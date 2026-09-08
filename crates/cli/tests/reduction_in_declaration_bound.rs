//! A reduction operator inside a DECLARATION BOUND sizes the declaration — §3 ⑦.
//!
//! The six §11.4.14 operators folded in the value domain since §4.5.382, but only
//! through `param_i64_at_declared`, which a declaration WITH a width reaches. Every
//! position that asks the plain module-scope walk — a packed range, an unpacked
//! dimension, a port range, an untyped parameter's initializer — got `None`, and the
//! bound consumers read `None` as ONE BIT at exit 0: `wire [((|4'b1010)+1):0] x;`
//! was 1 bit where iverilog and verilator both declare 3, and an `input` sized that
//! way truncated its actual across the module boundary. A 320-cell census (4
//! provenances × 9 positions × 6 operators, plus 60 edge shapes) counted 117
//! silent cells and 43 loud ones; a second 102-cell census covered constant-function
//! locals, the `-G` and `#()` override channels, the width-aware walk's own arms
//! (`!`, `?:`, `**`, a parameter SELECT under the reduction) and the untyped
//! parameter's type.
//!
//! Every value here was measured on iverilog 13.0 AND verilator 5.050 unless a test
//! says otherwise.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, args: &[&str]) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_rdb_{}_{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .args(args)
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

fn run(src: &str) -> (String, Option<i32>) {
    run_args(src, &[])
}

/// A `top` module around `decls` whose initial block displays `disp` once.
fn top(decls: &str, disp: &str) -> String {
    format!(
        "`timescale 1ns/1ns\npackage pk; localparam logic [7:0] K = 8'b1010_0110; endpackage\n\
         module top;\n{decls}\n  initial begin\n    $display({disp});\n    #1 $finish;\n  \
         end\nendmodule\n"
    )
}

fn prints(decls: &str, disp: &str, want: &str) {
    let (out, code) = run(&top(decls, disp));
    assert_eq!(code, Some(0), "exit for {decls}\n{out}");
    assert!(
        out.lines().any(|l| l == want),
        "expected line `{want}` for {decls}\n{out}"
    );
}

fn loud(decls: &str, disp: &str, fragment: &str) {
    let (out, code) = run(&top(decls, disp));
    assert_ne!(code, Some(0), "expected a refusal for {decls}\n{out}");
    assert!(
        out.contains(fragment),
        "expected `{fragment}` in the refusal for {decls}\n{out}"
    );
}

const SIX: [&str; 6] = ["&", "~&", "|", "~|", "^", "~^"];
/// `op 4'b1010` for the six operators, in `SIX` order.
const SIX_OF_1010: [u32; 6] = [0, 1, 1, 0, 0, 1];

/// The headline cell — a reduction in a PACKED range, on every operator, at both ends
/// of the range, and the same six in an unpacked dimension (both spellings).
#[test]
fn a_reduction_sizes_a_packed_range_and_an_unpacked_dimension() {
    for (op, v) in SIX.iter().zip(SIX_OF_1010) {
        prints(
            &format!("  wire [({op}4'b1010)+2:0] x;"),
            "\"%0d\", $bits(x)",
            &(v + 3).to_string(),
        );
        // The LOW bound: `[3:1]` is 3 bits, `[3:0]` is 4.
        prints(
            &format!("  wire [3:({op}4'b1010)] x;"),
            "\"%0d\", $bits(x)",
            &(4 - v).to_string(),
        );
        prints(
            &format!("  logic x [({op}4'b1010)+2:0];"),
            "\"%0d\", $size(x)",
            &(v + 3).to_string(),
        );
        prints(
            &format!("  logic x [({op}4'b1010)+2];"),
            "\"%0d\", $size(x)",
            &(v + 2).to_string(),
        );
    }
}

/// A PORT range is the cell that crosses a module boundary: the 1-bit port truncated
/// a 12-bit actual to `0` at exit 0. Both the child's own literal and a parameter the
/// parent overrides.
#[test]
fn a_reduction_sizes_a_port_across_the_module_boundary() {
    let src = "`timescale 1ns/1ns\n\
        module c #(parameter Q = 4'b0000) (input [(|Q)+7:0] p, input [(&4'b1111)+7:0] q);\n  \
        initial #0 $display(\"%0d %h %0d %h\", $bits(p), p, $bits(q), q);\nendmodule\n\
        module top; wire [11:0] a = 12'hABC; c #(.Q(4'b1010)) u(.p(a), .q(a)); \
        initial #1 $finish; endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.lines().any(|l| l == "9 0bc 9 0bc"), "{out}");
}

/// The operand may be any DECLARED provenance: a sized-literal parameter, a typed
/// localparam, a package constant, an unsized decimal, a parent `#()` override and a
/// `-G` override. Each carries a width the wide domain can read (`param_range`).
#[test]
fn a_reduction_reads_its_operand_at_every_declared_provenance() {
    prints(
        "  parameter P = 4'b1010;\n  wire [(|P)+2:0] x;",
        "\"%0d\", $bits(x)",
        "4",
    );
    prints(
        "  parameter P = 4'b1010;\n  wire [(&P)+2:0] x;",
        "\"%0d\", $bits(x)",
        "3",
    );
    prints(
        "  localparam logic [3:0] T = 4'b1010;\n  wire [(~^T)+2:0] x;",
        "\"%0d\", $bits(x)",
        "4",
    );
    prints("  wire [(^pk::K)+2:0] x;", "\"%0d\", $bits(x)", "3");
    prints("  wire [(|pk::K[7:4])+2:0] x;", "\"%0d\", $bits(x)", "4");
    prints(
        "  parameter D = 10;\n  wire [(~|D)+2:0] x;",
        "\"%0d\", $bits(x)",
        "3",
    );
    // `#()` override on a child, and `-G` on the top: the OVERRIDE's bits reduce.
    let src = "`timescale 1ns/1ns\nmodule c #(parameter P = 4'b1010) ();\n  \
        wire [(|P)+2:0] x;\n  initial #0 $display(\"%0d\", $bits(x));\nendmodule\n\
        module top; c #(.P(4'b0000)) u(); initial #1 $finish; endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.lines().any(|l| l == "3"), "{out}");
    let src = "`timescale 1ns/1ns\nmodule top #(parameter P = 4'b1010) ();\n  \
        wire [(|P)+2:0] x;\n  initial begin $display(\"%0d\", $bits(x)); #1 $finish; end\n\
        endmodule\n";
    let (out, code) = run_args(src, &["-G", "P=0"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.lines().any(|l| l == "3"), "{out}");
}

/// A bound holding a parameter SELECT is folded by the width-aware walk
/// (`const_range_bound_fold`), and so are the operands of `!`, a ternary condition and
/// a `**` exponent — that walk had no reduction arm, so every one of these was 1 bit.
#[test]
fn a_select_under_a_reduction_takes_the_width_aware_walk() {
    prints(
        "  parameter P = 4'b1010;\n  wire [(|P[3:1])+2:0] x;",
        "\"%0d\", $bits(x)",
        "4",
    );
    prints(
        "  parameter P = 4'b0001;\n  wire [(|P[3:1])+2:0] x;",
        "\"%0d\", $bits(x)",
        "3",
    );
    prints(
        "  parameter P = 4'b1010;\n  wire [(&P[3:1])+2:0] x;",
        "\"%0d\", $bits(x)",
        "3",
    );
    prints(
        "  parameter P = 4'b1010;\n  wire [(|P[2])+2:0] x;",
        "\"%0d\", $bits(x)",
        "3",
    );
    prints("  wire [(!(|4'b1010))+2:0] x;", "\"%0d\", $bits(x)", "3");
    prints("  wire [(!(&4'b1010))+2:0] x;", "\"%0d\", $bits(x)", "4");
    prints(
        "  wire [((|4'b1010) ? 5 : 3):0] x;",
        "\"%0d\", $bits(x)",
        "6",
    );
    prints(
        "  wire [((&4'b1010) ? 5 : 3):0] x;",
        "\"%0d\", $bits(x)",
        "4",
    );
    prints("  wire [(2**(|4'b1010))+1:0] x;", "\"%0d\", $bits(x)", "4");
    prints("  wire [(2**(&4'b1010))+1:0] x;", "\"%0d\", $bits(x)", "3");
    // A generate-if over a select under a reduction takes the right branch.
    prints(
        "  parameter P = 4'b1010;\n  if (&P[3:1]) begin : g initial $display(\"T\"); end \
         else begin : g initial $display(\"F\"); end",
        "\"-\"",
        "F",
    );
}

/// An UNTYPED parameter initialised by a reduction has the reduction's TYPE — one bit,
/// unsigned (§6.20.2 · §11.8.1) — not the 32 bits the value-inferred tail records for
/// any other expression. `$bits(R)` is 1, a bound built on `R` and a replication of it
/// both see one bit. The six values, then one chain.
#[test]
fn an_untyped_parameter_takes_the_reduction_type() {
    for (op, v) in SIX.iter().zip(SIX_OF_1010) {
        prints(
            &format!("  localparam R = {op}4'b1010;"),
            "\"%0d %0d\", R, $bits(R)",
            &format!("{v} 1"),
        );
    }
    prints(
        "  localparam R = |4'b1010;\n  wire [R+2:0] x;\n  localparam logic [7:0] S = {8{R}};",
        "\"%0d %0d %0d %h\", R, $bits(R), $bits(x), S",
        "1 1 4 ff",
    );
    prints(
        "  localparam R = &4'b1010;\n  wire [R+2:0] x;\n  localparam logic [7:0] S = {8{R}};",
        "\"%0d %0d %0d %h\", R, $bits(R), $bits(x), S",
        "0 1 3 00",
    );
    // `!` is the same type fact (§11.4.7): one bit, unsigned — over a reduction and
    // over anything else. `!(|4'b1010)` was loud; `$bits(!4'b1010)` was 32.
    prints(
        "  localparam R = !(|4'b1010);\n  localparam R3 = !4'b0000;\n  \
         localparam logic [7:0] T = {8{R3}};\n  wire [R3+2:0] y;",
        "\"%0d %0d %0d %0d %h %0d\", R, $bits(R), R3, $bits(R3), T, $bits(y)",
        "0 1 1 1 ff 4",
    );
    // A package-scope and a generate-scope untyped localparam, same rule. (`$bits` of
    // a hierarchical generate-scope name is unsupported on its own, so the generate
    // cell shows its one bit through a replication instead.)
    let src = "`timescale 1ns/1ns\npackage q; localparam R = ~&4'b1010; endpackage\n\
        module top;\n  if (1) begin : g localparam G = ^4'b1011; \
        localparam logic [7:0] S = {8{G}}; end\n  initial begin \
        $display(\"%0d %0d %0d %h\", q::R, $bits(q::R), g.G, g.S); #1 $finish; end\n\
        endmodule\n";
    let (out, code) = run(src);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.lines().any(|l| l == "1 1 1 ff"), "{out}");
}

/// A context-determined operator OVER a reduction, in an untyped parameter, now binds
/// the value BOTH oracles bind — at one bit.
///
/// It was deliberately LOUD from §4.5.407 to here: the value-inferred tail sized such
/// an initializer at 32 bits, so folding it would have printed 4294967294, 4 and
/// 4294967295 where the oracles print 0, 0 and 1, and `param_init_kept_loud` declined
/// at the four value sites rather than take that loud→silent-wrong trade. §4.5.460 gave
/// the tail the Table 11-21 arm, which sizes these at one bit, and this slice made the
/// VALUE lane width-aware to match — so the guard's own stated expiry condition was met
/// and it is gone from all four sites.
///
/// ⚠️ Both halves were needed. Deleting the guard against the OLD unlimited-then-coerce
/// value lane was measured to turn 8 further cells loud→silent-wrong: truncation
/// commutes with `~ - << & |` (which is why these three would have been fine) but not
/// with `/ % >> >>>`, so `(8'hFF + (|4'b1010)) >> 1` would have printed 128 where both
/// oracles print 0.
#[test]
fn a_narrow_context_over_a_reduction_binds_at_one_bit_in_an_untyped_parameter() {
    // The three headline cells, and their `$bits`. Both oracles, five binders
    // (module `localparam`, module-body `parameter`, ANSI header, package, generate).
    for (init, want) in [
        ("~(|4'b1010)", "0 1"),
        ("(|4'b1010) << 2", "0 1"),
        ("-(|4'b1010)", "1 1"),
        // A binary context-determined top over TWO reductions.
        ("(~&4'b1010) + (^4'b1010)", "1 1"),
    ] {
        prints(
            &format!("  localparam R = {init};"),
            "\"%0d %0d\", R, $bits(R)",
            want,
        );
    }
    // ⚠️ The one cell where the oracles differ in VALUE, kept as its own record:
    // verilator says 0 at one bit and iverilog says 2 at two. iverilog is disqualified
    // HERE by self-contradiction inside one design — its own
    // `$bits((|4'b1010) + 1'b1)` is 1 and its own expression value is 0, while it binds
    // the parameter at 2. vita lands on verilator, and on its own runtime spelling.
    prints(
        "  localparam R = (|4'b1010) + 1'b1;",
        "\"%0d %0d\", R, $bits(R)",
        "0 1",
    );
    prints(
        "  localparam logic [1:0] R1 = ~(|4'b1010);\n  localparam logic [3:0] R2 = (|4'b1010) << 2;\n  \
         localparam logic [1:0] R3 = -(|4'b1010);",
        "\"%0d %0d %0d\", R1, R2, R3",
        "2 4 3",
    );
    prints(
        "  localparam R = (|4'b1010) * 3;",
        "\"%0d %0d\", R, $bits(R)",
        "3 32",
    );
    prints(
        "  localparam R = (|4'b1010) ? 5 : 3;",
        "\"%0d %0d\", R, $bits(R)",
        "5 32",
    );
}

/// The BINDER census the guard's five call sites demanded: the same three initializers
/// answer the same way at every default binder, because they all reach one funnel
/// (`param_decl_width_unoverridden` + `eval_param_init`). Both oracles print exactly
/// this, and the `generate` binder additionally stops emitting a 4-error cascade
/// (`undeclared net/variable top.g[0].GA`) that the refused value used to leave behind.
#[test]
fn every_default_binder_binds_the_reduction_top_alike() {
    let (out, code) = run("package pk;\n  localparam PA = ~(|4'b1010);\nendpackage\n\
         module sub #(parameter MP = -(|4'b1010)) ();\n\
        \x20 initial $display(\"MP %0d %0d\", MP, $bits(MP));\n\
         endmodule\n\
         module hdr #(parameter HP = (|4'b1010) << 2) ();\n\
        \x20 initial $display(\"HP %0d %0d\", HP, $bits(HP));\n\
         endmodule\n\
         module top;\n  import pk::*;\n  localparam LA = ~(|4'b1010);\n\
        \x20 parameter  BP = ~(|4'b1010);\n\
        \x20 sub u0 ();\n  hdr u1 ();\n\
        \x20 generate if (1) begin : g\n\
        \x20   localparam GA = -(|4'b1010);\n\
        \x20   initial $display(\"GA %0d %0d\", GA, $bits(GA));\n\
        \x20 end endgenerate\n\
        \x20 initial begin\n\
        \x20   $display(\"LA %0d %0d\", LA, $bits(LA));\n\
        \x20   $display(\"BP %0d %0d\", BP, $bits(BP));\n\
        \x20   $display(\"PA %0d %0d\", pk::PA, $bits(pk::PA));\n\
        \x20   #1 $finish; end\n\
         endmodule\n");
    assert_eq!(code, Some(0), "{out}");
    for want in ["LA 0 1", "BP 0 1", "PA 0 1", "GA 1 1", "MP 1 1", "HP 0 1"] {
        assert!(
            out.lines().any(|l| l == want),
            "binder census line `{want}` (both oracles):\n{out}"
        );
    }
}

/// A constant function reduces its own FORMAL at the formal's declared width — and a
/// formal that SHADOWS a module parameter of another width reduces the formal, not the
/// parameter (`g`'s 4-bit `P`, not the module's 8-bit one).
#[test]
fn a_constant_function_reduces_its_own_formal() {
    prints(
        "  function automatic int f(input [3:0] v); return (|v) + 2; endfunction\n  \
         localparam F = f(4'b1010);\n  wire [F:0] w;",
        "\"%0d %0d\", F, $bits(w)",
        "3 4",
    );
    prints(
        "  function automatic int f(input [3:0] v); return (&v) + 2; endfunction\n  \
         localparam F = f(4'b1010);",
        "\"%0d\", F",
        "2",
    );
    prints(
        "  parameter P = 8'b1111_1111;\n  function automatic int g(input [3:0] P); \
         return (&P) + 2; endfunction\n  localparam G = g(4'b1010);",
        "\"%0d\", G",
        "2",
    );
    prints(
        "  parameter P = 8'b1111_1111;\n  function automatic int g(input [3:0] P); \
         return (~&P) + 2; endfunction\n  localparam G = g(4'b1010);",
        "\"%0d\", G",
        "3",
    );
}

/// An operand whose width was INFERRED from a value — `localparam E = 4'hF | 4'h0;`
/// is 4 bits in both oracles and 32 in `param_meta` — cannot be reduced soundly
/// (`&E` depends on it), and the bound used to clamp to a SILENT 1 bit where both
/// oracles declare 4. It is loud now, and says why.
#[test]
fn a_value_inferred_operand_width_is_loud_not_one_bit() {
    loud(
        "  localparam E = 4'hF | 4'h0;\n  wire [(&E)+2:0] x;",
        "\"%0d\", $bits(x)",
        "a reduction of an operand whose width the constant domain cannot read",
    );
    loud(
        "  localparam E = 4'hF | 4'h0;\n  localparam R = ^E;",
        "\"%0d\", R",
        "a reduction of an operand whose width the constant domain cannot read",
    );
    // ⚠️ Review F1: an ASCENDING `[0:3]` or non-zero-LSB `[7:4]` declaration IS a
    // declared width, and the wide domain still declines it (positional, direction-
    // free) — both oracles size these 4; PRE was a silent 1 bit; they are loud now and
    // the sentence must name that case rather than tell the author to add a range.
    for decl in [
        "parameter [0:3] P = 4'b1010;",
        "parameter [7:4] P = 4'b1010;",
    ] {
        loud(
            &format!("  {decl}\n  wire [(|P)+2:0] x;"),
            "\"%0d\", $bits(x)",
            "declared ascending `[0:N]` / with a non-zero low bound",
        );
    }
    // A net inside the operand is still the net's own message, not this one.
    loud(
        "  logic [3:0] n;\n  wire [(|n)+2:0] x;",
        "\"%0d\", $bits(x)",
        "a reference to net/variable `n`",
    );
}
