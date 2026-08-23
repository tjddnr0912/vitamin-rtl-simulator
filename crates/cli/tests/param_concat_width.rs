//! A concatenation's WIDTH was inferred from its value, not read off its operands.
//!
//! §11.4.12 makes a concatenation self-determined: its width is the sum of its
//! operands' own widths. That width is not recoverable from the value — `{2{32'd2}}`
//! is 64 bits wide and 34 bits of magnitude — but the untyped-parameter fallthrough in
//! `param_decl_width_opt` sized it from the folded i64 as `min_signed_bits(v).max(32)`
//! and recorded **35**. Both oracles say 64. That is a pre-existing silent-wrong: a
//! `$bits` or a part-select over that width reads bits that are not there.
//!
//! ⚠️ The arm that fixes it is gated twice, and both gates were written after a review
//! measured what happens without them — see the tests below for each.
//!
//! Every expected value here was measured live on iverilog 13.0 and verilator 5.050.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_catw_{}_{n}", std::process::id()));
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

fn expect(body: &str, inst: &str, want: &str) {
    let src = format!(
        "module m;\n  {body}\n  initial begin $display(\"VAL=%0d/%0d\", P, $bits(P)); $finish; end\n\
         endmodule\nmodule tb; {inst} endmodule\n"
    );
    let (out, code) = run(&src);
    assert_eq!(code, Some(0), "expected exit 0\n{body}\n{out}");
    assert!(
        out.contains(&format!("VAL={want}")),
        "expected {want}\n{body}\n{out}"
    );
}

/// The headline. 64 bits wide, 34 bits of magnitude; the inferred width was 35.
#[test]
fn a_replication_records_its_own_width_not_its_values() {
    expect("parameter P = {2{32'd2}};", "m u();", "8589934594/64");
}

#[test]
fn a_concatenation_records_the_sum_of_its_operand_widths() {
    expect("parameter P = {8'd1, 8'd2};", "m u();", "258/16");
    expect("parameter P = {2{2'd1}};", "m u();", "5/4");
    expect("parameter P = {8'd1, {2{8'd2}}};", "m u();", "66050/24");
}

/// A parenthesised concatenation is the same value and must get the same width. The
/// peel loop above the arm also strips unary `+`/`-`, which is a separate question, so
/// the arm peels parens itself. Without this, `{2{8'h1}}` recorded 16 and
/// `({2{8'h1}})` recorded 32 — one value, two answers.
#[test]
fn a_parenthesised_concatenation_gets_the_same_width() {
    expect("parameter P = ({2{8'h1}});", "m u();", "257/16");
}

/// A DECLARED range still wins: the arm sits with the type-determined family, after
/// the range check, not before it.
#[test]
fn a_declared_range_still_wins() {
    expect(
        "parameter [95:0] P = {2{32'd2}};",
        "m u();",
        "8589934594/96",
    );
    expect("parameter integer P = {2{8'd1}};", "m u();", "257/32");
}

/// ⚠️⚠️ Gate one. IEEE §6.20.2 gives an untyped parameter the range of its FINAL
/// OVERRIDE value. Keying the width on the declared expression truncated the override:
/// `#(parameter P = {2{8'h1}})` overridden with `32'hDEADBEEF` came out 16 bits holding
/// `beef`, where both oracles keep 32 bits and `deadbeef`. A declared TYPE legitimately
/// survives an override; a self-determined initializer expression does not, because the
/// value it was determined from has been replaced.
#[test]
fn an_override_keeps_its_own_width_not_the_defaults() {
    let src = |inst: &str| {
        format!(
            "module m #(parameter P = {{2{{8'h1}}}}) ();\n  \
             initial begin $display(\"VAL=%0d/%0h\", $bits(P), P); $finish; end\nendmodule\n\
             module tb; {inst} endmodule\n"
        )
    };
    let (out, code) = run(&src("m #(.P(32'hDEADBEEF)) u();"));
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("VAL=32/deadbeef"),
        "the override keeps its width\n{out}"
    );

    // `.P()` with no value legally means "keep the default", so the default binds.
    let (out, code) = run(&src("m #(.P()) u();"));
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("VAL=16/101"),
        "an empty override keeps the default\n{out}"
    );

    // ...and with no override at all, the concatenation's own width binds.
    let (out, code) = run(&src("m u();"));
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("VAL=16/101"),
        "the default binds at its own width\n{out}"
    );
}

/// The same defect one step worse: the truncated width would size a NET.
#[test]
fn an_override_that_sizes_a_net_is_not_truncated() {
    let (out, code) = run(
        "module m #(parameter P = {2{8'h1}}) ();\n  wire [P-1:0] bus; assign bus = 0;\n  \
         initial $display(\"VAL=%0d\", $bits(bus));\nendmodule\n\
         module tb; m #(.P({4{8'h2}})) u(); endmodule\n",
    );
    assert_ne!(
        code,
        Some(0),
        "must not silently build a 514-bit bus\n{out}"
    );
}

/// ⚠️⚠️ Gate two, and it has to be proved LEAF BY LEAF. The resolver behind this arm
/// sizes a NAME from `param_meta` — where value-INFERRED widths are recorded — and
/// guesses `(32, false)` when there is none, so it cannot report provenance at all.
/// Answering `declared_only` unconditionally made a concatenation a laundering wrapper
/// around exactly the provenance that flag fences off: it declared a **263-bit** net
/// where iverilog declares 1, which is the §4.5.363 regression re-entered through the
/// concat door. A concatenation of SIZED LITERALS does state its width; a leaf that is
/// a name does not.
#[test]
fn a_concatenation_over_a_name_is_not_declared_provenance() {
    let (out, code) = run(
        "module tb;\n  localparam W = ~8'hCB;\n  localparam Q = {W};\n  \
         logic [(W[15:8])+8-1:0] u;\n  logic [(Q[15:8])+8-1:0] v;\n  \
         initial $display(\"VAL=%0d/%0d\", $bits(u), $bits(v));\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("VAL=1/1"),
        "both oracles declare 1 bit each\n{out}"
    );
}

/// Recorded residue, NOT a target: a select over a concatenation parameter is still
/// silently 1 bit. `localparam W = {2{8'h09}}; logic [W[7:0]-1:0] v;` declares 1 where
/// iverilog declares 9 — and it did before this slice too (PRE `1/32`, POST `1/16`),
/// because the select path needs the recorded RANGE (`param_sel_range`, §4.5.363), not
/// just the width. The slice fixes `$bits(W)` 32 → 16 and leaves the bound where it
/// was. Pinned so that closing the range side turns this red and asks for promotion.
#[test]
fn a_select_over_a_concatenation_parameter_is_still_a_one_bit_bound() {
    let (out, code) = run(
        "module tb;\n  localparam W = {2{8'h09}};\n  logic [W[7:0]-1:0] v; assign v = 0;\n  \
         initial $display(\"VAL=%0d/%0d\", $bits(v), $bits(W));\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert!(
        out.contains("VAL=1/16"),
        "iverilog says 9/16; if the bound is no longer 1, promote this row\n{out}"
    );
}

/// Recorded residue, NOT a target: the replication COUNT is still literal-only, so
/// `{N{32'd2}}` — how every parameterised AXI/Ethernet core writes a per-port
/// parameter vector — stays loud. Widening it was BUILT, MEASURED and REVERTED in the
/// same slice: it needs the count folded in the right SCOPE (a constant-function local
/// is not a constant expression, and the module-scope evaluator resolves past the
/// shadow to a same-named parameter), with a DEPTH the module-scope evaluator restarts
/// at 0 (a call in the count overflowed the stack), and it re-opens the select-bound
/// gap above from the other side. See ROADMAP §3 ② for the measured mechanism.
#[test]
fn a_parameter_replication_count_is_still_loud() {
    let (out, code) = run(
        "module tb;\n  parameter N=2;\n  localparam P = {N{32'd2}};\n  \
         initial $display(\"VAL=%0d\", P);\nendmodule\n",
    );
    assert_ne!(
        code,
        Some(0),
        "if this now folds, the count widened — see ROADMAP §3 ②\n{out}"
    );
}
