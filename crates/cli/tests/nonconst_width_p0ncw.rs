//! P0-NCW: a declared net/array range bound that references a NET/variable or a
//! HIERARCHICAL name is not a constant expression. The old elaborate path folded
//! such a bound to 0 and SILENTLY produced a width-1 net (a P0 silent-wrong);
//! iverilog 13.0 rejects it ("A reference to a net or variable / a hierarchical
//! reference is not allowed in a constant expression"). vita must now be LOUD
//! (E3009 / exit 1), NOT a silent value. Pure IR-0 (elaborate lowering only):
//! every VALID parameterized/localparam/genvar width still folds exactly, so the
//! golden stays byte-identical.
//!
//! A const-but-unfoldable bound that carries NO net/hier ref — e.g. a constant
//! function call `f(3)`, which iverilog DOES accept — is intentionally NOT made
//! loud here (vita simply cannot fold it yet); that avoids a false-loud and is a
//! separate, narrower follow-on. Loud verdicts are pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ncw_{}_{n}", std::process::id()));
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

fn is_loud(out: &str, code: Option<i32>) -> bool {
    out.contains("VITA-E3009") || code == Some(1)
}

// ── LOUD: non-constant bounds (iverilog rejects each) ──────────────────────

#[test]
fn net_referenced_packed_width_is_loud() {
    // reg [sig-1:0] x — `sig` is a net: not a constant expression.
    let (out, code) = run("module t;\n\
         reg [7:0] sig;\n\
         reg [sig-1:0] x;\n\
         initial $display(\"B %0d\", $bits(x));\n\
         endmodule\n");
    assert!(
        is_loud(&out, code),
        "net-referenced packed width must be loud: {out} code={code:?}"
    );
}

#[test]
fn net_referenced_unpacked_dim_is_loud() {
    // reg x [sig-1:0] — non-constant unpacked dimension.
    let (out, code) = run("module t;\n\
         reg [7:0] sig;\n\
         reg x [sig-1:0];\n\
         initial $display(\"hi\");\n\
         endmodule\n");
    assert!(
        is_loud(&out, code),
        "net-referenced unpacked dim must be loud: {out} code={code:?}"
    );
}

#[test]
fn net_referenced_size_dim_is_loud() {
    // reg x [sig] — the `[n]` size form with a net bound is also non-constant.
    let (out, code) = run("module t;\n\
         reg [7:0] sig;\n\
         reg x [sig];\n\
         initial $display(\"hi\");\n\
         endmodule\n");
    assert!(
        is_loud(&out, code),
        "net-referenced size dim must be loud: {out} code={code:?}"
    );
}

#[test]
fn hierarchical_param_in_const_width_is_loud() {
    // reg [dut.W-1:0] x — a hierarchical reference is not allowed in a constant
    // expression even though dut.W is itself a parameter (iverilog rejects).
    let (out, code) = run("module sub #(parameter W=5) (); endmodule\n\
         module top; sub dut();\n\
           reg [dut.W-1:0] x;\n\
           initial $display(\"B %0d\", $bits(x));\n\
         endmodule\n");
    assert!(
        is_loud(&out, code),
        "hierarchical param in const width must be loud: {out} code={code:?}"
    );
}

#[test]
fn hierarchical_net_in_const_width_is_loud() {
    let (out, code) = run("module sub; reg [7:0] s; endmodule\n\
         module top; sub dut();\n\
           reg [dut.s-1:0] x;\n\
           initial $display(\"hi\");\n\
         endmodule\n");
    assert!(
        is_loud(&out, code),
        "hierarchical net in const width must be loud: {out} code={code:?}"
    );
}

#[test]
fn net_referenced_multidim_packed_outer_is_loud() {
    // reg [sig-1:0][7:0] x — the non-constant OUTER packed dim is loud.
    let (out, code) = run("module t;\n\
         reg [3:0] sig;\n\
         reg [sig-1:0][7:0] x;\n\
         initial $display(\"hi\");\n\
         endmodule\n");
    assert!(
        is_loud(&out, code),
        "net-referenced multi-dim packed width must be loud: {out} code={code:?}"
    );
}

// ── VALID: every constant source still folds (NO false-loud, exact width) ───

#[test]
fn parameter_width_still_folds() {
    let (out, _c) = run("module t;\n\
         parameter W=5;\n\
         reg [W-1:0] x;\n\
         initial $display(\"OK %0d\", $bits(x));\n\
         endmodule\n");
    assert!(out.contains("OK 5"), "param width must fold to 5:\n{out}");
}

#[test]
fn localparam_derived_width_still_folds() {
    let (out, _c) = run("module t;\n\
         parameter W=4;\n\
         localparam B=W*2;\n\
         reg [B-1:0] x;\n\
         initial $display(\"OK %0d\", $bits(x));\n\
         endmodule\n");
    assert!(
        out.contains("OK 8"),
        "localparam-derived width must fold to 8:\n{out}"
    );
}

#[test]
fn clog2_param_width_still_folds() {
    // reg [$clog2(N)-1:0] — N=16 ⇒ $clog2=4 ⇒ width 4.
    let (out, _c) = run("module t;\n\
         parameter N=16;\n\
         reg [$clog2(N)-1:0] x;\n\
         initial $display(\"OK %0d\", $bits(x));\n\
         endmodule\n");
    assert!(
        out.contains("OK 4"),
        "$clog2 param width must fold to 4:\n{out}"
    );
}

#[test]
fn genvar_width_in_generate_still_folds() {
    // a genvar-parameterized width inside a generate-for is constant per
    // iteration: with i=2, `reg [i:0] x` is 3 bits wide. Observed via an
    // in-scope `$bits(x)` (an indexed-segment `g[2].x` external access is a
    // separate, unrelated parser follow-on).
    let (out, _c) = run("module t;\n\
         genvar i;\n\
         generate for (i=2;i<3;i=i+1) begin:g\n\
           reg [i:0] x;\n\
           initial $display(\"OK %0d\", $bits(x));\n\
         end endgenerate\n\
         endmodule\n");
    assert!(
        out.contains("OK 3"),
        "genvar width must fold per iteration to 3:\n{out}"
    );
}

#[test]
fn parameter_unpacked_dim_still_folds() {
    // reg x [D-1:0] — a param-sized unpacked array must keep working.
    let (out, _c) = run("module t;\n\
         parameter D=4;\n\
         reg [7:0] mem [D-1:0];\n\
         initial begin mem[3]=8'haa; $display(\"OK %h\", mem[3]); end\n\
         endmodule\n");
    assert!(
        out.contains("OK aa"),
        "param unpacked dim must keep working:\n{out}"
    );
}

// ── DIAGNOSTIC HONESTY: a LITERAL negative low bound `[3:-2]` is a distinct
//    (not-yet-supported) shape from the `[W-1:0]`-with-W==0 parameter underflow.
//    Full support is a loud→supported gap (ROADMAP §3); the value pinned here is
//    the HONEST diagnostic — behaviour is unchanged (both shapes warn + clamp to
//    width 1). The old unified guard misdiagnosed the literal case as
//    "parameterized range underflowed (param value 0?)".

fn run_stderr(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_ncwe_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.code(),
    )
}

#[test]
fn literal_negative_low_bound_is_sized_and_selectable() {
    // `logic [3:-2]` used to warn and clamp to width 1 (whole-value damage). It is sized
    // `|msb-lsb|+1` now, stored normalized as `[w-1:0]` because `NetVar.msb`/`lsb` are
    // frozen `u32`, with the declared bound in the `net_decl_neg_lsb` side map so a bit
    // select addresses the declared numbering. iverilog: 6 bits, `101010`, `x[3]`=1,
    // `x[-2]`=0, `$left`=3, `$right`=-2.
    let (err, _c) = run_stderr(
        "module t; logic [3:-2] x;\n\
         initial begin x = 6'b101010;\n\
           $display(\"B %0d %b %b %b %0d %0d\", $bits(x), x, x[3], x[-2], $left(x), $right(x));\n\
         end endmodule\n",
    );
    assert!(
        !err.contains("negative packed-range low bound"),
        "the clamp warning must be gone:\n{err}"
    );
    let (out, _) = run("module t; logic [3:-2] x;\n\
         initial begin x = 6'b101010;\n\
           $display(\"B %0d %b %b %b %0d %0d\", $bits(x), x, x[3], x[-2], $left(x), $right(x));\n\
         end endmodule\n");
    assert!(out.contains("B 6 101010 1 0 3 -2"), "{out}");
}

#[test]
fn a_part_select_of_a_negative_low_bound_net_is_loud_for_the_right_reason() {
    // The remaining gap: a PART select folds its own bounds through the UNSIGNED
    // `const_eval_u32`, where `-2` reads as 0xFFFFFFFE and used to trip a direction
    // check that blamed the wrong thing. Loud with the real reason instead.
    let (err, c) = run_stderr(
        "module t; logic [3:-2] x;\n\
         initial begin x = 6'b101010; $display(\"%b\", x[1:-2]); end endmodule\n",
    );
    assert_ne!(c, Some(0), "must stay loud:\n{err}");
    assert!(
        err.contains("PART select of a net declared with a negative low bound"),
        "must name the real reason:\n{err}"
    );
}

#[test]
fn param_zero_width_underflow_keeps_its_message() {
    // The degenerate `[W-1:0]` with W==0 (only the HIGH bound folds negative, low
    // bound stays 0) is the parameter-underflow shape — it must keep its own
    // message and stay graceful (non-fatal width-1; cf. elaborate v3_12).
    let (err, _c) = run_stderr(
        "module t; parameter W=0; logic [W-1:0] x;\n\
         initial $display(\"B %0d\", $bits(x)); endmodule\n",
    );
    assert!(
        err.contains("parameterized range underflowed"),
        "parameter underflow must keep its distinct message:\n{err}"
    );
    assert!(
        !err.contains("negative packed-range low bound"),
        "param underflow (lsb=0) is not the literal-negative shape:\n{err}"
    );
}
