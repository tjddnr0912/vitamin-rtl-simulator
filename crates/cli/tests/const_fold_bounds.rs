//! Select BOUNDS and replication COUNTS are constant expressions (IEEE 1800
//! §11.4.12.2 / §11.5.1), but vita folded them with two much weaker folders than
//! the one it uses for parameters: the literal-only free `const_eval_u32`
//! (IntLit / Paren / unary ±) and the engine's shallow `const_u32_of_expr`
//! (Const, the Add/Sub of a width tree, `$clog2` of a Const). Every other
//! constant form — `*`, `/`, `%`, `**`, shifts, a ternary, a cast, a
//! constant-function call, `$clog2` of an expression, `$bits(x)/k`, a
//! package-scoped param — degraded SILENTLY (exit 0, no diagnostic):
//!
//!   * a part-select READ collapsed to a 1-bit width (`v[int'(11):int'(8)]` = 0),
//!   * a part-select WRITE clobbered the whole net above `lsb` (`16'hFFFF` with
//!     `w[int'(7):int'(4)] = 4'h0` became `000f`, not `ff0f`),
//!   * an ascending-net select read 0,
//!   * a multi-dim packed outer select read 0 and wrote the wrong bits,
//!   * a replication count became an empty 0-width result.
//!
//! The fix routes every such site through one funnel, `const_bound_u32`, which
//! tries the literal folder first (so every shape that already worked keeps its
//! exact IR) and then the full elaborate const domain. Because that domain is
//! width-UNLIMITED while SV constant arithmetic wraps at its self-determined
//! width, it is consulted only for expressions `const_fold_is_width_exact`
//! proves cannot wrap — otherwise `$clog2(4'd15 + 4'd15)` would fold 30 → 5
//! where SV truncates to 14 → 4, i.e. a WRONG NON-ZERO answer replacing an
//! empty one. Every value below is pinned to LIVE iverilog 13.0.
use std::fmt::Write as _;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_cfb_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code(),
    )
}

/// Header shared by the equivalence sweeps: one value (4, 8, 11 …) reachable
/// through every constant form the const domain can fold.
const PRE: &str = "package pk; parameter ONE = 1; endpackage\n\
     module m;\n\
       import pk::*;\n\
       parameter P1 = 1, P4 = 4, P8 = 8, P11 = 11;\n\
       function automatic int fid(input int x); fid = x; endfunction\n\
       function automatic int fret(input int x); return x; endfunction\n\
       logic [31:0] bw32;\n\
       logic [15:0] v = 16'b1010_1100_0011_0101;\n\
       logic [0:15] va = 16'b1010_1100_0011_0101;\n\
       logic [15:0] w;\n\
       logic [3:0][7:0] mp;\n\
       logic [7:0] mem [0:3];\n\
       initial begin\n\
         bw32 = 0; mp = {8'hAA, 8'hBB, 8'hCC, 8'hDD};\n\
         mem[0] = 8'h11; mem[1] = 8'h22;\n";
const POST: &str = "  #1 $finish;\n  end\nendmodule\n";

/// Assert every line of `out` that starts with the tag prefix carries `want`.
/// Also fails when NOTHING was printed — a whole result silently disappearing is
/// the most dangerous regression shape, and a value-only scan would miss it.
fn all_eq(out: &str, tag: &str, want: &str, ctx: &str) {
    let lines: Vec<&str> = out.lines().filter(|l| l.starts_with(tag)).collect();
    assert!(
        !lines.is_empty(),
        "{ctx}: NO `{tag}` output at all (silent drop); got:\n{out}"
    );
    for l in &lines {
        assert!(
            l.ends_with(want),
            "{ctx}: expected `{want}` on `{l}`; full output:\n{out}"
        );
    }
}

/// Every constant form for the SAME value must produce the SAME part-select as
/// the literal one — a vita-internal equivalence differential that needs no
/// oracle to be meaningful — and that shared value is the one iverilog prints.
#[test]
fn part_select_bounds_fold_every_const_form() {
    // `v[11:8]` of 16'b1010_1100_0011_0101 is 4'b1100 = c (iverilog: c).
    let forms = [
        ("11", "8"),
        ("P11", "P8"),
        ("(11*1)", "(8*1)"),
        ("(22/2)", "(16/2)"),
        ("(11%16)", "(8%16)"),
        ("(11**1)", "(8**1)"),
        ("(1?11:0)", "(1?8:0)"),
        ("int'(11)", "int'(8)"),
        ("longint'(11)", "longint'(8)"),
        ("fid(11)", "fid(8)"),
        ("fret(11)", "fret(8)"),
        ("$clog2(2048)", "$clog2(256)"),
        ("($bits(bw32)/32*11)", "($bits(bw32)/32*8)"),
        ("(pk::ONE*11)", "(pk::ONE*8)"),
        ("((11<<1)>>1)", "((8<<1)>>1)"),
    ];
    let mut body = String::new();
    for (m, l) in forms {
        let _ = writeln!(body, "    $display(\"A=%h\", v[{m}:{l}]);");
    }
    let (out, c) = run(&format!("{PRE}{body}{POST}"));
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    all_eq(
        &out,
        "A=",
        "c",
        "descending part-select bound, every const form",
    );
}

/// The round-20 external report's CRITICAL repro, verbatim. It was filed against a
/// commit that predates this slice, and it is the lane-slicing spelling real RTL uses
/// (`[2*W-1:W]`, `[N*W-1:(N-1)*W]`) rather than the literal-bound forms above — so it
/// is pinned separately, in the report's own words, with the report's own values.
///
/// The failure was as bad as it gets: no diagnostic, and the 64-bit upper lane came
/// back as ONE BIT (`d[W]`, zero-extended). It made a production SHA-3 core produce a
/// silently wrong digest for every message of 9 bytes or more, because messages of 8
/// or fewer never touch the upper lane — which is why nineteen rounds of KAT testing
/// never saw it. Every line here matches live iverilog 13.0.
#[test]
fn a_multiplied_parameter_bound_selects_the_full_lane() {
    let (out, c) = run("module t;\n\
           localparam int W = 64;\n\
           localparam int H = 128;\n\
           logic [127:0] d = 128'hAABBCCDDEEFF0011_2233445566778899;\n\
           logic [2*W-1:0] v;\n\
           initial begin\n\
             $display(\"B=%h\", d[127:64]);\n\
             $display(\"B=%h\", d[127:W]);\n\
             $display(\"B=%h\", d[H-1:W]);\n\
             $display(\"B=%h\", d[W+63:W]);\n\
             $display(\"B=%h\", d[W+W-1:W]);\n\
             $display(\"B=%h\", d[W +: W]);\n\
             $display(\"B=%h\", d[2*W-1 -: W]);\n\
             $display(\"B=%h\", d[2*W-1:W]);\n\
             $display(\"B=%h\", d[2*64-1:64]);\n\
             $display(\"B=%h\", d[W*2-1:W]);\n\
             $display(\"B=%h\", d[W/1*2-1:W]);\n\
             $display(\"B=%h\", d[(W<<1)-1:W]);\n\
             $display(\"B=%h\", d[2*W-1:2*W-64]);\n\
             $display(\"N=%0d\", $bits(v));\n\
             $finish;\n\
           end\n\
         endmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    all_eq(
        &out,
        "B=",
        "aabbccddeeff0011",
        "the report's lane-select table",
    );
    assert!(
        out.contains("N=128"),
        "declared width path too; got:\n{out}"
    );
}

/// The WRITE twin of the read above: a bound that did not fold used to widen the
/// write to the whole net above `lsb`, silently destroying bits 15:12.
#[test]
fn part_select_write_bounds_fold_every_const_form() {
    // 16'hFFFF with [11:8] cleared is f0ff (iverilog: f0ff). The old behavior
    // widened the write to everything above `lsb` and produced 000f.
    let forms = [
        ("11", "8"),
        ("(11*1)", "(8*1)"),
        ("int'(11)", "int'(8)"),
        ("fid(11)", "fid(8)"),
        ("(1?11:0)", "(1?8:0)"),
        ("$clog2(2048)", "$clog2(256)"),
    ];
    let mut body = String::new();
    for (m, l) in forms {
        let _ = writeln!(
            body,
            "    w = 16'hFFFF; w[{m}:{l}] = 4'h0; $display(\"W=%h\", w);"
        );
    }
    let (out, c) = run(&format!("{PRE}{body}{POST}"));
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    all_eq(
        &out,
        "W=",
        "f0ff",
        "part-select WRITE bound, every const form",
    );
}

/// An ASCENDING net takes `[lo:hi]`, and its width can only come from the folded
/// bounds (the arena `msb - lsb` tree underflows), so a non-literal bound read a
/// silent 0 there rather than a narrow value.
#[test]
fn ascending_net_part_select_folds_const_bounds() {
    // va is `logic [0:15]`; va[4:7] of 1010_1100_0011_0101 is 4'b1100 = c.
    let body = "    $display(\"C=%h\", va[4:7]);\n\
                \x20   $display(\"C=%h\", va[int'(4):int'(7)]);\n\
                \x20   $display(\"C=%h\", va[fid(4):fid(7)]);\n\
                \x20   $display(\"C=%h\", va[(4*1):(7*1)]);\n";
    let (out, c) = run(&format!("{PRE}{body}{POST}"));
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    all_eq(&out, "C=", "c", "ascending-net part-select");
}

/// Multi-dim packed: the outer-element select and the leaf bit-range each had
/// their own literal-only fold, so both read/wrote silently wrong data.
#[test]
fn md_packed_outer_and_leaf_bounds_fold() {
    // mp = {AA,BB,CC,DD} over `logic [3:0][7:0]`: mp[2:1] = bbcc, mp[2][7:4] = b.
    let body = "    $display(\"O=%h\", mp[2:1]);\n\
                \x20   $display(\"O=%h\", mp[int'(2):int'(1)]);\n\
                \x20   $display(\"O=%h\", mp[fid(2):fid(1)]);\n\
                \x20   $display(\"L=%h\", mp[2][7:4]);\n\
                \x20   $display(\"L=%h\", mp[2][int'(7):int'(4)]);\n\
                \x20   $display(\"I=%h\", mp[1+:2]);\n\
                \x20   $display(\"I=%h\", mp[int'(1)+:2]);\n\
                \x20   $display(\"M=%h\", mem[1][7:4]);\n\
                \x20   $display(\"M=%h\", mem[1][int'(7):int'(4)]);\n";
    let (out, c) = run(&format!("{PRE}{body}{POST}"));
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    all_eq(&out, "O=", "bbcc", "md-packed outer part-select");
    all_eq(&out, "L=", "b", "md-packed leaf part-select");
    all_eq(&out, "I=", "bbcc", "md-packed indexed part-select");
    all_eq(&out, "M=", "2", "array-element part-select");
}

/// A replication count that did not reduce in the engine's shallow fold became a
/// 0-width result. Every form below is 3 copies of `1'b1` in iverilog.
#[test]
fn replication_count_folds_every_const_form() {
    let forms = [
        "3",
        "P1+2",
        "(3*1)",
        "(6/2)",
        "(19%16)",
        "(3**1)",
        "(1?3:0)",
        "int'(3)",
        "longint'(3)",
        "fid(3)",
        "fret(3)",
        "$clog2(8)",
        "$clog2(P4+4)",
        "($bits(bw32)/32*3)",
        "(pk::ONE*3)",
        "((3<<1)>>1)",
    ];
    let mut body = String::new();
    for f in forms {
        let _ = writeln!(body, "    $display(\"R=%b\", {{{f}{{1'b1}}}});");
    }
    let (out, c) = run(&format!("{PRE}{body}{POST}"));
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    all_eq(&out, "R=", "111", "replication count, every const form");
}

/// The width guard, pinned by the case that would otherwise break: SV evaluates
/// `4'd15 + 4'd15` at 4 bits (14, so `$clog2` = 4) while the i64 const domain
/// says 30 (`$clog2` = 5). Folding that would put a WRONG NON-ZERO count where
/// an empty one used to be, so `const_fold_is_width_exact` declines and the old
/// value stands; the same arithmetic at ≥32-bit width folds and is right.
#[test]
fn narrow_operand_arithmetic_declines_instead_of_folding_wrong() {
    let (out, c) = run("module m; logic [63:0] g, h;\n\
         initial begin\n\
           g = {$clog2(4'd15 + 4'd15){1'b1}};\n\
           h = {$clog2(15 + 15){1'b1}};\n\
           $display(\"G=%0d %0d\", $countones(g), $countones(h));\n\
           $display(\"V=%0d\", 4'd15 + 4'd15);\n\
           #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0), "the decline is silent, not loud; got:\n{out}");
    // Narrow operands ⇒ decline ⇒ the pre-existing empty count (0), NEVER the
    // wrong 5. Wide operands ⇒ fold ⇒ $clog2(30) = 5, which is what iverilog
    // prints. The 0 is the tracked self-width residual, not a new defect.
    assert!(
        out.contains("G=0 5"),
        "narrow operands decline, wide ones fold; got:\n{out}"
    );
    // The RUNTIME is unaffected and still truncates at 4 bits.
    assert!(out.contains("V=14"), "runtime 4-bit add; got:\n{out}");
}

/// Documented residual, pinned so a future width-aware const fold has a marker:
/// a bare `+` of narrow sized literals reaches the ENGINE's own `Add` fold,
/// which is equally width-unlimited. This is PRE-EXISTING (identical before and
/// after this slice) — the guard's job was only to stop the other operators from
/// joining it, and iverilog's answer is 14, not 30.
#[test]
fn bare_narrow_add_count_is_the_preexisting_engine_fold_residual() {
    let (out, c) = run(
        "module m; logic [63:0] a; initial begin a = {(4'd15 + 4'd15){1'b1}};\n\
         $display(\"N=%0d\", $countones(a)); #1 $finish; end endmodule\n",
    );
    assert_eq!(c, Some(0));
    assert!(
        out.contains("N=30"),
        "pre-existing engine-side Add fold (iverilog says 14); got:\n{out}"
    );
}

/// A cast is now a foldable constant expression everywhere the const domain is
/// consulted, not only in bound/count position.
#[test]
fn integral_cast_folds_in_const_contexts() {
    let (out, c) = run("module m;\n\
           localparam int WC = int'(7);\n\
           localparam int WB = byte'(7);\n\
           localparam int WS = 8'(7);\n\
           logic [int'(7):0] wide;\n\
           initial begin\n\
             wide = '1;\n\
             $display(\"K=%0d %0d %0d %0d\", WC, WB, WS, $bits(wide));\n\
             #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0), "casts fold, no diagnostic; got:\n{out}");
    assert!(
        out.contains("K=7 7 7 8"),
        "int'/byte'/size cast fold + cast as a declared range bound; got:\n{out}"
    );
}

/// A reversed constant part-select is rejected by iverilog ("out of order").
/// vita already said so for literal bounds; the same bounds behind a cast or a
/// constant function used to slip through to a silent 1-bit read. Loud is the
/// correct verdict, and it must be loud for BOTH spellings.
#[test]
fn reversed_const_bounds_are_loud_for_every_form() {
    for bounds in ["4:7", "int'(4):int'(7)", "fid(4):fid(7)"] {
        let (out, c) = run(&format!(
            "module m; function automatic int fid(input int x); fid = x; endfunction\n\
             logic [15:0] v; initial begin v = 16'hABCD;\n\
             $display(\"X=%h\", v[{bounds}]); #1 $finish; end endmodule\n"
        ));
        assert_ne!(c, Some(0), "reversed `{bounds}` must be loud; got:\n{out}");
    }
}

/// The `[c +: w]` / `[c -: w]` WIDTH is the same kind of constant expression and
/// had the same weak fold on the plain-vector and hierarchical paths, read and
/// write alike — a cast/call/ternary width read a silent 1 bit or 0.
#[test]
fn indexed_part_select_width_folds_every_const_form() {
    let (out, c) = run("module t; parameter W = 8; logic [31:0] v, u;\n\
         function automatic int w8(); w8 = 8; endfunction\n\
         initial begin v = 32'hDEADBEEF;\n\
           $display(\"P=%h\", v[8 +: 8]);\n\
           $display(\"P=%h\", v[8 +: int'(8)]);\n\
           $display(\"P=%h\", v[8 +: w8()]);\n\
           $display(\"P=%h\", v[8 +: (W>4?8:4)]);\n\
           $display(\"P=%h\", v[15 -: int'(8)]);\n\
           u = 32'hFFFFFFFF; u[8 +: int'(8)] = 8'h00; $display(\"U=%h\", u);\n\
           #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0), "no diagnostics expected; got:\n{out}");
    all_eq(
        &out,
        "P=",
        "be",
        "indexed part-select width, every const form",
    );
    all_eq(&out, "U=", "ffff00ff", "indexed part-select WRITE width");
}

/// Guard conditions that adversarial differential probing proved necessary. Each
/// line here is a value the width-unlimited i64 fold gets WRONG, so the fold must
/// decline and leave the previous (already correct, or already loud) behavior.
#[test]
fn width_guard_declines_every_unprovable_shape() {
    // (a) 32-bit leaves but an intermediate above 32 bits: SV drops the high bits
    // (`(1<<33)>>30` = 0) while i64 says 8. The old width-1 fallback IS iverilog's
    // answer here, so folding would turn a CORRECT select into a wrong one — and
    // the reversed-looking `v[7 : …]` would become a bogus "out of order" reject.
    let (out, c) = run(
        "module t; logic [31:0] v; initial begin v = 32'hDEADBEEF;\n\
           $display(\"A=%h\", v[((32'd1 << 32'd33) >> 32'd30) : 0]);\n\
           $display(\"B=%h\", v[7 : ((32'd1 << 32'd33) >> 32'd30)]);\n\
           #1 $finish; end endmodule\n",
    );
    assert_eq!(c, Some(0), "must NOT become a false loud; got:\n{out}");
    // `A=1` is iverilog's answer (the bound is 0, so this is `v[0:0]`) — folding
    // would have made it `0ef`. `B=1` is the unchanged pre-existing width-1 read
    // (iverilog gives `ef`); the point of this line is that it is still SILENT and
    // identical to before, not the "out of order" reject the wrong fold produced.
    assert!(out.contains("A=1"), "iverilog gives 1; got:\n{out}");
    assert!(
        out.contains("B=1"),
        "unchanged, and not a false loud; got:\n{out}"
    );

    // (b) a constant FUNCTION whose body does narrow arithmetic: SV computes
    // `(8'd200 + 8'd100) >> 2` at 8 bits (44 >> 2 = 11); the interpreter's i64 says
    // 75. A `byte` return is below 32 bits, so the call declines rather than
    // folding a wrong count. A call must never short-circuit the leaf check.
    let (out, c) = run(
        "module t; function automatic byte g8(); g8 = (8'd200 + 8'd100) >> 2; endfunction\n\
         initial begin $display(\"G=%0d\", $countones(64'({g8(){1'b1}}))); \
         #1 $finish; end endmodule\n",
    );
    assert_eq!(c, Some(0));
    assert!(
        out.contains("G=0"),
        "narrow-return const fn declines (never 75); got:\n{out}"
    );

    // (b2) a WIDE return does not rescue a narrow LOCAL: the interpreter never
    // coerces an assignment to its declared width, so `bit [3:0] t = 4'd15+4'd15`
    // is 30 there and 14 in SV. The whole signature plus every body decl must be
    // ≥32 bits — vita's RUNTIME already prints 14, which is what makes the fold's
    // 30 a silent-wrong rather than a shared imprecision.
    let (out, c) = run(
        "module t; function automatic int f(); bit [3:0] tt; tt = 4'd15 + 4'd15; return tt; \
         endfunction\n\
         initial begin $display(\"C=%0d\", $countones(64'({f(){1'b1}}))); \
         $display(\"R=%0d\", f()); #1 $finish; end endmodule\n",
    );
    assert_eq!(c, Some(0));
    assert!(
        out.contains("C=0") && out.contains("R=14"),
        "narrow-local const fn declines (never 30); runtime stays 14; got:\n{out}"
    );

    // (c) a negative literal bound. `const_eval_u32` folds `-1` by wrapping_neg to
    // 0xFFFF_FFFF, which passes the descending direction check — the width must not
    // be computed from it (`0xFFFF_FFFF - 0 + 1` overflowed u32 and PANICKED).
    let (out, c) = run(
        "module m; logic [7:0] x, r; initial begin x = 8'hA5; r = x[-1:0]; \
         $display(\"N=%b\", r); #1 $finish; end endmodule\n",
    );
    assert_eq!(c, Some(0), "must not panic (was exit 101); got:\n{out}");
    assert!(
        out.contains("N=00000001"),
        "unchanged behavior; got:\n{out}"
    );
}

/// The cast fold made `int'(…)` reachable at every `const_eval_in_scope` caller,
/// including untyped parameter binding — where the value's SIGNEDNESS decides how
/// it materializes. Without the matching `const_expr_signed` arm this bound −300
/// as unsigned and printed 4294966996.
#[test]
fn cast_folded_param_keeps_its_signedness() {
    let (out, c) = run(
        "module m; localparam P = int'(-300); localparam Q = int'(300);\n\
         initial begin $display(\"S=%0d %b %b\", P, P < 0, Q > -1); #1 $finish; end endmodule\n",
    );
    assert_eq!(c, Some(0));
    assert!(
        out.contains("S=-300 1 1"),
        "signed cast binds signed (iverilog: -300 1 1); got:\n{out}"
    );
}

/// A folded value is read by THREE predicates — the fold itself, the signedness,
/// and the WIDTH. Widening only the first two made an untyped `localparam PL =
/// longint'(-1)` bind at 32 bits: wrong `$bits`, wrong `%h`, and a concatenation
/// of the wrong LENGTH. A cast initializer is type-determined (§6.24).
#[test]
fn cast_folded_param_keeps_its_declared_width() {
    let (out, c) = run("module m;\n\
           localparam PL = longint'(-1); localparam PBY = byte'(-1);\n\
           localparam PS = shortint'(-1); localparam PT = time'(-1);\n\
           localparam byte TY = byte'(-1);\n\
           initial begin\n\
             $display(\"B=%0d %0d %0d %0d\", $bits(PL), $bits(PBY), $bits(PS), $bits(PT));\n\
             $display(\"H=%h %h\", PL, PBY);\n\
             $display(\"K=%h\", {PBY, 8'hAA});\n\
             $display(\"T=%0d %h\", $bits(TY), TY);\n\
             #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    // Every line is iverilog's, including the 2-byte concat (it was 5 bytes).
    assert!(
        out.contains("B=64 8 16 64"),
        "declared cast widths; got:\n{out}"
    );
    assert!(out.contains("H=ffffffffffffffff ff"), "got:\n{out}");
    assert!(
        out.contains("K=ffaa"),
        "concat LENGTH follows the cast; got:\n{out}"
    );
    assert!(
        out.contains("T=8 ff"),
        "a DECLARED type still wins; got:\n{out}"
    );
}

/// The const-function width check has to be TRANSITIVE and see NESTED decls —
/// `eval_const_call` binds decls in nested blocks and `for` inits too, and a wide
/// wrapper would otherwise launder a narrow callee. Both folded wrong non-zero
/// counts before; both must decline to the previous (empty) behavior.
#[test]
fn const_fn_width_check_is_transitive_and_sees_nested_decls() {
    // wide wrapper `int f(); f = g8();` around a `byte` callee — iverilog says 11.
    let (out, c) = run(
        "module m; function byte g8(); g8 = (8'd200 + 8'd100) >> 2; endfunction\n\
         function int f(); f = g8(); endfunction\n\
         logic [63:0] r; initial begin r = {f(){1'b1}};\n\
         $display(\"W=%0d\", $countones(r)); #1 $finish; end endmodule\n",
    );
    assert_eq!(c, Some(0));
    assert!(
        out.contains("W=0"),
        "wrapper must not launder (never 75); got:\n{out}"
    );

    // a narrow decl inside a NAMED BLOCK, invisible to `f.body_decls`.
    let (out, c) = run(
        "module m; function int f(); begin : inner bit [3:0] t; t = 4'd15 + 4'd15; f = t; end \
         endfunction\n\
         logic [63:0] r; initial begin r = {f(){1'b1}};\n\
         $display(\"N=%0d\", $countones(r)); #1 $finish; end endmodule\n",
    );
    assert_eq!(c, Some(0));
    assert!(
        out.contains("N=0"),
        "nested decl must decline (never 30); got:\n{out}"
    );

    // CONTROL: a plain all-wide const function still folds — the guard must not
    // have swallowed the capability this slice exists to deliver.
    let (out, c) = run(
        "module m; function automatic int fid(input int x); fid = x; endfunction\n\
         logic [15:0] v = 16'b1010_1100_0011_0101;\n\
         initial begin $display(\"OK=%h %b\", v[fid(11):fid(8)], {fid(3){1'b1}}); \
         #1 $finish; end endmodule\n",
    );
    assert_eq!(c, Some(0));
    assert!(
        out.contains("OK=c 111"),
        "wide const fn still folds; got:\n{out}"
    );
}

/// A real-valued bound or count has no integral constant value in this domain.
/// `const_eval_cast` folds its operand with `const_eval_in_scope`, which does not
/// model reals, so a real never sneaks through the new cast arm as an integer —
/// these stay LOUD exactly as they were.
#[test]
fn real_bounds_and_counts_stay_loud() {
    for expr in ["{int'(R){1'b1}}", "v[R:0]", "{R{1'b1}}"] {
        let (out, c) = run(&format!(
            "module m; parameter real R = 3.5; logic [15:0] v; logic [63:0] q;\n\
             initial begin v = 16'hABCD; q = 0; $display(\"Y=%h\", {expr}); \
             #1 $finish; end endmodule\n"
        ));
        assert_ne!(c, Some(0), "real in `{expr}` must be loud; got:\n{out}");
    }
}

/// PRE-EXISTING and unchanged by this slice (measured identical on a PRE build):
/// an `int'(<real param>)` BOUND is a legal integral expression that iverilog
/// evaluates (`v[int'(3.5):0]` = `v[4:0]`), but vita reads a silent 1-bit slice.
/// The value is unreachable here because the const domain has no real arithmetic
/// — the tracked `real` const-fold work (ROADMAP §0 T2) is its prerequisite, so
/// this pins the CURRENT behavior rather than asserting correctness.
#[test]
fn int_cast_of_real_param_bound_is_a_tracked_preexisting_gap() {
    let (out, c) = run("module m; parameter real R = 3.5; logic [15:0] v;\n\
         initial begin v = 16'hABCD; $display(\"Y=%h\", v[int'(R):0]); \
         #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("Y=1"),
        "tracked gap: iverilog gives 0d (v[4:0]); vita still reads 1 bit; got:\n{out}"
    );
}
