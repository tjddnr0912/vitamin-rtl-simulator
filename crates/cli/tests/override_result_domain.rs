//! An override's own TYPE types an untyped parameter (IEEE 1800-2017 §6.20.2: "if the
//! expression is real, the parameter is real"; §11.8.1 for which expressions are real;
//! Table 11-21 for an integral top's width) — ROADMAP §2 "Real" (the two override bullets)
//! and the `-G` decimal bullet.
//!
//! The binder decided an untyped target's type from its DEFAULT and from the typed override
//! channels, and none of them records whether the override's result is real: `by_name` is an
//! i64, and a real parameter with an exactly integral value folds to one. So `#(.P(X))` with
//! `localparam real X = 5` bound `parameter P = 3` as the integer 5. The collectors now
//! record the REAL fold of a real-by-construction override (`Elaborator::override_real`),
//! which also replaces the i64 a real target bound (`X / 2` folded to 2). The operator
//! channel states the type of a prim cast and a comparison / logical / reduction top
//! (`Elaborator::override_top_meta`), which types an integral override of a real default
//! (`#(.R(X > 1))` stayed real) and sizes the same override of an integral default (32 bits
//! for a comparison). A call is left alone: its return range belongs to the declaring scope.
//! A `-G` decimal states its 32-bit signed type.
//!
//! Every value is what iverilog 13.0 and verilator 5.052 both print unless a cell says
//! otherwise; `PRE` is what vita printed before.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn lines_args(src: &str, args: &[&str]) -> Vec<String> {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_ord_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let s =
        String::from_utf8_lossy(&out.stdout).into_owned() + &String::from_utf8_lossy(&out.stderr);
    assert!(out.status.success(), "expected exit 0, got:\n{s}");
    let mut v: Vec<String> = s
        .lines()
        .filter(|l| l.starts_with("T "))
        .map(|l| l.trim_end().to_string())
        .collect();
    v.sort();
    v
}

fn check(src: &str, want: &[&str]) {
    check_args(src, &[], want);
}

fn check_args(src: &str, args: &[&str], want: &[&str]) {
    let mut w: Vec<String> = want.iter().map(|s| s.to_string()).collect();
    w.sort();
    assert_eq!(lines_args(src, args), w, "{args:?}\n{src}");
}

const SUB: &str =
    "module sub #(parameter P = 3) ();\n  initial #1 $display(\"T %m %f\", P/4);\nendmodule\n";
const RSUB: &str =
    "module rsub #(parameter R = 2.5) ();\n  initial #1 $display(\"T %m %f\", R/2);\nendmodule\n";

/// A real override makes an untyped integral default real, on every binder and spelling.
/// PRE: `1.000000` for every 1.25, `0.000000` for `X / 2`, `-1.000000`, `6.000000`.
#[test]
fn a_real_override_makes_an_untyped_parameter_real() {
    let src = format!(
        "{SUB}module sub2;\n  parameter P = 3;\n  initial #1 $display(\"T %m %f\", P/4);\nendmodule\n\
         interface ifc #(parameter P = 3);\n  initial #1 $display(\"T %m %f\", P/4);\nendinterface\n\
         package pk;\n  localparam real X = 5;\nendpackage\n\
         module t;\n  localparam real X = 5;\n  sub #(.P(X)) a();\n  sub #(X) b();\n  sub #(.P(X / 2)) c();\n  \
         sub #(.P(-X)) d();\n  sub #(.P(1 ? X : 2)) e();\n  sub #(.P(pk::X)) f();\n  sub g();\n  \
         defparam g.P = X;\n  sub2 #(.P(X)) h();\n  ifc #(.P(X)) i();\n  if (1) begin : gs\n    \
         sub #(.P(X ** 2)) j();\n  end\n  initial #3 $finish;\nendmodule\n"
    );
    check(
        &src,
        &[
            "T t.a 1.250000",
            "T t.b 1.250000",
            "T t.c 0.625000",
            "T t.d -1.250000",
            "T t.e 1.250000",
            "T t.f 1.250000",
            "T t.g 1.250000",
            "T t.h 1.250000",
            "T t.i 1.250000",
            "T t.gs.j 6.250000",
        ],
    );
}

/// …and forwards as a real: the child's untyped `Q` takes the forwarded real too. A parent
/// `parameter real Q = 5` (an exact value with an i64 twin) is the same override.
/// PRE: `1.000000` for both.
#[test]
fn a_forwarded_real_stays_real() {
    let src = format!(
        "module leaf #(parameter Q = 1) ();\n  initial #1 $display(\"T %m %f\", Q/4);\nendmodule\n\
         module mid #(parameter P = 3) ();\n  leaf #(.Q(P)) l();\nendmodule\n{SUB}\
         module t #(parameter real Q = 5);\n  localparam real X = 5;\n  mid #(.P(X)) m();\n  \
         sub #(.P(Q)) s();\n  initial #3 $finish;\nendmodule\n"
    );
    check(&src, &["T t.m.l 1.250000", "T t.s 1.250000"]);
}

/// An integral override makes an untyped REAL default integral when its top states its type:
/// a prim cast, a comparison over a real, a byte cast of a wide value. PRE: `1.500000`,
/// `0.500000`, `-100.000000`. (A CALL keeps the default's route — its return range belongs to
/// the declaring scope; ROADMAP §2 "Real".)
#[test]
fn an_integral_override_makes_an_untyped_real_default_integral() {
    let src = format!(
        "{RSUB}module t;\n  localparam real X = 5;\n  rsub #(.R(int'(2.5))) c();\n  \
         rsub #(.R(X > 1)) d();\n  rsub #(.R(byte'(200))) f();\n  initial #3 $finish;\nendmodule\n"
    );
    check(
        &src,
        &["T t.c 1.000000", "T t.d 0.000000", "T t.f -28.000000"],
    );
    // The type is the override's: PRE 64 (the real default) for each.
    let src = "module s #(parameter Q = 2.5) ();\n  logic [63:0] V;\n  initial begin V = Q; #1 \
               $display(\"T %m %0d %h\", $bits(Q), V); end\nendmodule\nmodule t;\n  \
               localparam real X = 5;\n  s #(.Q(int'(2.5))) b();\n  s #(.Q(X > 1)) e();\n  \
               s #(.Q(-int'(3.0))) f();\n  initial #3 $finish;\nendmodule\n";
    check(
        src,
        &[
            "T t.b 32 0000000000000003",
            "T t.e 1 0000000000000001",
            "T t.f 32 fffffffffffffffd",
        ],
    );
}

/// The same tops onto an INTEGRAL default: the target took the default's 32 bits. PRE: `32`
/// for each. (The default is a value no override here equals: verilator binds an override
/// equal to the default at the default's width, ROADMAP §2 Oracle splits.)
#[test]
fn an_integral_top_states_its_own_width() {
    let src = "module s #(parameter Q = 7) ();\n  initial #1 $display(\"T %m %0d %0d\", $bits(Q), \
               Q);\nendmodule\nmodule t;\n  function automatic logic [7:0] fl(input int a); \
               return a; endfunction\n  localparam real X = 5;\n  s #(.Q(X > 1)) b();\n  \
               s #(.Q(byte'(100))) c();\n  s #(.Q(!fl(0))) d();\n  initial #3 $finish;\n\
               endmodule\n";
    check(src, &["T t.b 1 1", "T t.c 8 100", "T t.d 1 1"]);
}

/// A real override's value is its REAL fold: the i64 of `X / 2` is 2, which a real target
/// bound as 2.0. PRE: `1.000000`, `1.000000`, `1.000000`, `2.000000`, `0.500000`.
#[test]
fn a_real_override_value_is_folded_in_the_real_domain() {
    let src = format!(
        "{RSUB}module rsubr #(parameter real R = 2.5) ();\n  initial #1 $display(\"T %m %f\", \
         R/2);\nendmodule\nmodule t;\n  localparam real X = 5;\n  localparam real X6 = 6;\n  \
         rsub #(.R(X / 2)) a();\n  rsubr #(.R(X / 2)) b();\n  rsub c();\n  defparam c.R = X / 2;\n  \
         rsubr #(.R(X / 2 * 2)) d();\n  rsubr #(.R(X6 / 4)) e();\n  initial #3 $finish;\nendmodule\n"
    );
    check(
        &src,
        &[
            "T t.a 1.250000",
            "T t.b 1.250000",
            "T t.c 1.250000",
            "T t.d 2.500000",
            "T t.e 0.750000",
        ],
    );
}

/// Consumers of an inexact real override (`X / 4` = 1.25), which binds no i64 view: a
/// generate condition, a `%0d` read, a loop bound, a typed derived localparam. PRE (the
/// integer 1): `le`, `1 1.000000`, `1`, `4`.
#[test]
fn an_inexact_real_override_reads_as_real() {
    let src = "module s #(parameter P = 3) ();\n  if (P > 1) begin : a\n    initial #1 \
               $display(\"T gt\");\n  end else begin : b\n    initial #1 $display(\"T le\");\n  \
               end\n  localparam [7:0] L = P * 4;\n  integer i, n;\n  initial begin n = 0; \
               for (i = 0; i < P; i++) n++; #1 $display(\"T %0d %f %0d %0d\", P, P, n, L); \
               end\nendmodule\nmodule t;\n  localparam real X = 5;\n  s #(.P(X / 4)) u();\n  \
               initial #3 $finish;\nendmodule\n";
    check(src, &["T 1 1.250000 2 5", "T gt"]);
}

/// A `-G` decimal is a 32-bit signed integer (§5.7.1) onto an untyped target, whatever the
/// default's type. PRE: `0004 0000 4 2`, `000f 000b 4 7`, `… 64 …`, and the real `1.500000`.
#[test]
fn a_g_decimal_states_its_32_bit_signed_type() {
    let tu = |d: &str| {
        format!(
            "module t #(parameter U = {d}) ();\n  logic [15:0] V;\n  initial begin V = U; #1 \
             $display(\"T %h %h %0d %h\", V, U - 16'd4, $bits(U), U >> 1); $finish; \
             end\nendmodule\n"
        )
    };
    check_args(
        &tu("4'd3"),
        &["-G", "U=100"],
        &["T 0064 00000060 32 00000032"],
    );
    check_args(
        &tu("4'd3"),
        &["-G", "U=-1"],
        &["T ffff fffffffb 32 7fffffff"],
    );
    check_args(
        &tu("64'd3"),
        &["-G", "U=100"],
        &["T 0064 00000060 32 00000032"],
    );
    check_args(
        &tu("4'sd3"),
        &["-G", "U=0"],
        &["T 0000 fffffffc 32 00000000"],
    );
    let body = "module t;\n  parameter U = 4'd3;\n  initial begin #1 $display(\"T %0d %0d\", U, \
                $bits(U)); $finish; end\nendmodule\n";
    check_args(body, &["-G", "U=100"], &["T 100 32"]);
    let r = "module t #(parameter R = 2.5) ();\n  initial begin #1 $display(\"T %f\", R/2); \
             $finish; end\nendmodule\n";
    check_args(r, &["-G", "R=3"], &["T 1.000000"]);
    check_args(r, &["-G", "R=-3"], &["T -1.000000"]);
}

/// The real fold reads a name's INNERMOST binding — the one the lowering reads and the
/// domain was classified by. An integral `localparam N = 3` in a generate block over a
/// module-scope `real N = 2.5`: PRE `7.500000`, `1.250000`, `1.750000` for the declarations
/// (an outer real read through the inner integer), and the override twins, which were right
/// by the integer fold, stay right.
#[test]
fn the_real_fold_reads_the_innermost_binding() {
    let src = format!(
        "{SUB}{RSUB}module t;\n  localparam real N = 2.5;\n  localparam real X = 5;\n  \
         localparam real H = 0.5;\n  if (1) begin : g\n    localparam N = 3;\n    \
         localparam real R1 = N + X;\n    localparam real R2 = N / 2;\n    \
         localparam real R3 = N / 2 + H;\n    initial #1 $display(\"T R %f %f %f\", R1, R2, R3);\n    \
         sub #(.P(N + X)) u();\n    rsub #(.R(N * X)) v();\n    sub #(.P(X - N)) w();\n  end\n  \
         initial #3 $finish;\nendmodule\n"
    );
    check(
        &src,
        &[
            "T R 8.000000 1.000000 1.500000",
            "T t.g.u 2.000000",
            "T t.g.v 7.500000",
            "T t.g.w 0.500000",
        ],
    );
}

/// A genvar hides an outer REAL of its name: `localparam real N = 2.5; for (genvar N …)`.
/// The override walks read the real map before `params`, so the real override lane bound
/// the outer 2.5 (`0.625000`, `1.250000`) until the loop suspended it.
#[test]
fn a_genvar_hides_an_outer_real_of_its_name() {
    let src = format!(
        "{SUB}{RSUB}module t;\n  localparam real N = 2.5;\n  for (genvar N = 0; N < 2; N++) \
         begin : gf\n    sub #(.P(N)) u();\n    rsub #(.R(N)) w();\n    sub #(.P(N * 5)) x();\n  \
         end\n  initial #3 $finish;\nendmodule\n"
    );
    check(
        &src,
        &[
            "T t.gf[0].u 0.000000",
            "T t.gf[0].w 0.000000",
            "T t.gf[0].x 0.000000",
            "T t.gf[1].u 0.000000",
            "T t.gf[1].w 0.000000",
            "T t.gf[1].x 1.000000",
        ],
    );
}

/// An imported package real is bound as its i64 twin alone (ROADMAP §3.a ⑨), so no walk can
/// tell its domain and the override keeps the route it had: onto a real default it stays real.
#[test]
fn an_imported_package_real_keeps_the_route_it_had() {
    let src = format!(
        "package pk;\n  localparam real X = 5;\nendpackage\n{RSUB}module t;\n  import pk::*;\n  \
         rsub #(.R(X)) a();\n  rsub #(.R(-X)) b();\n  rsub #(X) c();\n  initial #3 \
         $finish;\nendmodule\n"
    );
    check(
        &src,
        &["T t.a 2.500000", "T t.b -2.500000", "T t.c 2.500000"],
    );
}

/// A parameter a real override made real keeps the hierarchical twin the integer route
/// published, so `u.P` still reads (the module real route serves no hierarchical read).
#[test]
fn a_real_override_keeps_the_hierarchical_twin() {
    let src = format!(
        "{SUB}module t;\n  localparam real X = 5;\n  sub #(.P(X)) u();\n  initial #2 \
         $display(\"T hier %0d\", u.P);\n  initial #3 $finish;\nendmodule\n"
    );
    check(&src, &["T hier 5", "T t.u 1.250000"]);
}

/// `$realtobits` of an integral argument is the bits of its REAL value (§20.5): an untyped
/// real default an integral override made integral (`.R(3)`), a parameter, a signed and an
/// unsigned variable, a literal; a real argument is unchanged (`.R(fi(3))` keeps the default
/// real, a call being typed by no channel). PRE: the integer's own bits (`0000000000000005`, `00000000fffffffd`,
/// `00000000000000c8`, …).
#[test]
fn realtobits_converts_an_integral_argument() {
    let src = "module icons #(parameter R = 2.5) ();\n  initial #1 $display(\"T %m %h\", \
               $realtobits(R));\nendmodule\nmodule t;\n  function automatic int fi(input int \
               a); return a; endfunction\n  localparam P = 5;\n  integer i;\n  logic [7:0] b;\n  \
               logic signed [7:0] sb;\n  icons #(.R(fi(3))) a();\n  icons #(.R(3)) b0();\n  \
               icons c();\n  initial begin\n    i = -3; b = 8'd200; sb = -8'sd56;\n    #2 \
               $display(\"T %h %h %h %h %h %h %h\", $realtobits(P), $realtobits(i), \
               $realtobits(b), $realtobits(sb), $realtobits(3), $realtobits(2.5), \
               $realtobits(64'd9007199254740993));\n    $finish;\n  end\nendmodule\n";
    check(
        src,
        &[
            "T 4014000000000000 c008000000000000 4069000000000000 c04c000000000000 4008000000000000 4004000000000000 4340000000000000",
            "T t.a 4008000000000000",
            "T t.b0 4008000000000000",
            "T t.c 4004000000000000",
        ],
    );
}

/// …with each x/z bit read as 0 (§6.12.2), and a wide argument rounded as both oracles
/// round it: the set bits added LSB first, so a low bit absorbed before a tie is decided
/// does not break the tie (`2^180 + 2^127 + 1` is `2^180`). PRE: `000000000000000X`, the low
/// 64 bits.
#[test]
fn realtobits_reads_x_as_zero_and_rounds_as_the_oracles_do() {
    let src = "module t;\n  logic [7:0] xv;\n  logic [127:0] a;\n  logic [199:0] f;\n  \
               initial begin\n    xv = 8'b0000_001x;\n    a = (128'd1 << 120) | (128'd1 << 67) \
               | 128'd1;\n    f = (200'd1 << 180) | (200'd1 << 127) | 200'd1;\n    #1 \
               $display(\"T %h %h %h\", $realtobits(xv), $realtobits(a), $realtobits(f));\n    \
               $finish;\n  end\nendmodule\n";
    check(
        src,
        &["T 4000000000000000 4770000000000000 4b30000000000000"],
    );
}

/// ⚠️ The cells the slice must not move. A declared type survives an override (§6.20.2), an
/// integral result of a real operand stays integral, and a sized `-G` literal keeps its own
/// width. `parameter signed P` is an oracle split (iverilog binds the real override as the
/// integer 5, `1.000000`; verilator as the real, `1.250000`) that vita leaves on the side it
/// was on.
#[test]
fn the_cells_the_slice_must_not_move() {
    let src = "module s7 #(parameter int P = 3) ();\n  initial #1 $display(\"T %m %f\", \
               P/4);\nendmodule\nmodule s8 #(parameter [7:0] P = 3) ();\n  initial #1 \
               $display(\"T %m %f\", P/4);\nendmodule\nmodule s9 #(parameter signed P = 3) ();\n  \
               initial #1 $display(\"T %m %f\", P/4);\nendmodule\nmodule sub #(parameter P = 3) \
               ();\n  initial #1 $display(\"T %m %f\", P/4);\nendmodule\nmodule t;\n  \
               localparam real X = 5;\n  s7 #(.P(X)) a();\n  s8 #(.P(X)) b();\n  s9 #(.P(X)) \
               c();\n  sub #(.P(X > 1)) d();\n  sub #(.P($rtoi(X))) e();\n  sub #(.P(int'(X))) \
               f();\n  initial #3 $finish;\nendmodule\n";
    check(
        src,
        &[
            "T t.a 1.000000",
            "T t.b 1.000000",
            "T t.c 1.000000",
            "T t.d 0.000000",
            "T t.e 1.000000",
            "T t.f 1.000000",
        ],
    );
    let typed = "module t #(parameter [7:0] U = 3) ();\n  logic [15:0] V;\n  initial begin V = U; \
                 #1 $display(\"T %h %h %0d %h\", V, U - 16'd4, $bits(U), U >> 1); $finish; \
                 end\nendmodule\n";
    check_args(typed, &["-G", "U=100"], &["T 0064 0060 8 32"]);
    let sized = "module t #(parameter U = 4'd3) ();\n  initial begin #1 $display(\"T %0d %0d\", \
                 U, $bits(U)); $finish; end\nendmodule\n";
    check_args(sized, &["-G", "U=8'd100"], &["T 100 8"]);
}
