//! An unsized fill (`'0`/`'1`) in the i64 constant lane is sized by its CONTEXT, not
//! by the literal parser's 32-bit container — ROADMAP §2 🆕 C.
//!
//! `parse_int_literal` sizes a fill at a hard 32 (`explicit_width.unwrap_or(32)`), and
//! every constant-domain consumer that read a fill through it inherited that width:
//! `localparam U = '1 ^ 1'b0;` was 4294967295 with `$bits` 33, `$clog2('1)` was 32,
//! `$bits('1)` was 32, `-G U='1` bound −1 at 32 bits, and a right shift by `'1`
//! shifted by 4294967295 (everything out). Both oracles size a fill to its context
//! (§5.7.1) — one bit when nothing is around it. The fix gives the width-aware walk a
//! fill leaf (`eval_const_env_at`) and a fill self width of ZERO (`const_self_width`,
//! "takes the context"), routes an untyped parameter's fill-bearing initializer
//! through that walk (`untyped_fill_init`), types a fill override of an implicit
//! parameter as one unsigned bit, and answers `$bits` of a bare fill with 1.
//!
//! Every value here was measured on iverilog 13.0 AND verilator 5.050 unless a test
//! says otherwise; 93 cells in the slice's census.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run_args(src: &str, args: &[&str]) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_fcl_{}_{n}", std::process::id()));
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

fn top(decls: &str, disp: &str) -> String {
    format!(
        "`timescale 1ns/1ns\nmodule top;\n{decls}\n  initial begin\n    $display({disp});\n    \
         #1 $finish;\n  end\nendmodule\n"
    )
}

fn prints(decls: &str, disp: &str, want: &str) {
    let (out, code) = run_args(&top(decls, disp), &[]);
    assert_eq!(code, Some(0), "exit for {decls}\n{out}");
    assert!(
        out.lines().any(|l| l == want),
        "expected `{want}` for {decls}\n{out}"
    );
}

fn loud(decls: &str, disp: &str) {
    let (out, code) = run_args(&top(decls, disp), &[]);
    assert_ne!(code, Some(0), "expected a refusal for {decls}\n{out}");
}

/// ⓐ An UNTYPED parameter's fill-bearing initializer folds at the initializer's own
/// width: a lone fill is one bit, a fill beside a sibling takes the sibling's width.
#[test]
fn an_untyped_parameter_sizes_its_fill_to_the_initializer() {
    for (init, want) in [
        ("'1", "1 1"),
        ("'0", "0 1"),
        ("('1)", "1 1"),
        ("'1 ^ 1'b0", "1 1"),
        ("'1 & 4'hF", "15 4"),
        ("'1 | 8'h00", "255 8"),
        ("'1 >> 1", "0 1"),
        ("'1 << 1", "0 1"),
        ("'1 == 1'b1", "1 1"),
        ("1'b1 ? '1 : 1'b0", "1 1"),
        ("-'1", "1 1"),
        ("~'1", "0 1"),
        // `4'd8 - '1`: the fill is the 4-bit all-ones, 8 − 15 wraps to 9 (both oracles
        // 9; verilator sizes it 4, iverilog 5 — the width is pinned to verilator).
        ("4'd8 - '1", "9 4"),
        // A region of NOTHING but fills is one bit: 1 + 1 wraps to 0.
        ("'1 + '1", "0 1"),
        // Review: a fill beside a REDUCTION — the kept-loud guard for a narrow
        // context-determined top over a reduction must not refuse the fill route
        // (PRE printed 1 with `$bits` 32; POST briefly refused it).
        ("'1 & (|4'hF)", "1 1"),
        ("'1 | (|4'hF)", "1 1"),
        ("~('1 & (|4'hF))", "0 1"),
    ] {
        prints(
            &format!("  localparam U = {init};"),
            "\"%0d %0d\", U, $bits(U)",
            want,
        );
    }
    // Package and generate scopes take the same route.
    let src = "`timescale 1ns/1ns\npackage q; localparam U = '1 & 4'hF; endpackage\n\
        module top;\n  if (1) begin : g localparam G = '1 ^ 1'b0; \
        localparam logic [7:0] S = {8{G}}; end\n  initial begin \
        $display(\"%0d %0d %0d %h\", q::U, $bits(q::U), g.G, g.S); #1 $finish; end\n\
        endmodule\n";
    let (out, code) = run_args(src, &[]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.lines().any(|l| l == "15 4 1 ff"), "{out}");
}

/// ⓑ ⓒ `$clog2` and `$bits` of a fill-bearing argument in a constant context — the
/// runtime spellings already gave these answers.
#[test]
fn clog2_and_bits_of_a_fill_argument() {
    prints("  localparam int K = $clog2('1);", "\"%0d\", K", "0");
    prints("  localparam int K = $clog2('0);", "\"%0d\", K", "0");
    prints("  localparam int K = $clog2('1 / 8'd2);", "\"%0d\", K", "7");
    prints(
        "  localparam int K = $clog2('1 & 8'hFF);",
        "\"%0d\", K",
        "8",
    );
    prints("  localparam int K = $bits('1);", "\"%0d\", K", "1");
    // A fill under an operator has no `$bits` fold arm here and stays loud (both
    // oracles 4 / 1) — recorded, not pinned to a value.
    loud("  localparam int K = $bits('1 & 4'hF);", "\"%0d\", K");
}

/// ⓓ A fill OVERRIDE of an implicit parameter has the fill's type — one unsigned bit —
/// whatever the default literal was (verilator; iverilog cannot spell `-G U='1`). A
/// declared type keeps its width.
#[test]
fn a_fill_override_of_an_implicit_parameter_is_one_bit() {
    for decl in [
        "parameter U = 0",
        "parameter U = 8'h00",
        "parameter U = 4'sd1",
    ] {
        let src = format!(
            "`timescale 1ns/1ns\nmodule top #({decl}) ();\n  initial begin $display(\"%0d %0d\", \
             U, $bits(U)); #1 $finish; end\nendmodule\n"
        );
        let (out, code) = run_args(&src, &["-G", "U='1"]);
        assert_eq!(code, Some(0), "{out}");
        assert!(out.lines().any(|l| l == "1 1"), "[{decl}]\n{out}");
    }
    let src = "`timescale 1ns/1ns\nmodule top #(parameter logic [7:0] U = 0) ();\n  initial \
        begin $display(\"%h %0d\", U, $bits(U)); #1 $finish; end\nendmodule\n";
    let (out, code) = run_args(src, &["-G", "U='1"]);
    assert_eq!(code, Some(0), "{out}");
    assert!(out.lines().any(|l| l == "ff 8"), "{out}");
}

/// ⓔ A right shift by a fill amount shifts by ONE (the amount is a self-determined
/// position, one bit); the left shift was already right (§4.5.406). Typed and untyped.
#[test]
fn a_shift_by_a_fill_amount_is_a_shift_by_one() {
    prints(
        "  localparam logic [7:0] A = 8'hFF >> '1;",
        "\"%h\", A",
        "7f",
    );
    prints(
        "  localparam logic [7:0] A = 8'h0F << '1;",
        "\"%h\", A",
        "1e",
    );
    prints(
        "  localparam logic signed [7:0] A = 8'shF0 >>> '1;",
        "\"%h\", A",
        "f8",
    );
    prints(
        "  localparam A = 8'hFF >> '1;",
        "\"%h %0d\", A, $bits(A)",
        "7f 8",
    );
}

/// A fill in a comparison is sized against its sibling — a generate-if on `'1 == 1'b1`
/// took the ELSE branch at exit 0 (both oracles: then).
#[test]
fn a_fill_in_a_comparison_is_sized_against_its_sibling() {
    prints(
        "  if ('1 == 1'b1) begin : g initial $display(\"T\"); end else begin : g initial \
         $display(\"F\"); end",
        "\"-\"",
        "T",
    );
    prints(
        "  localparam logic [7:0] U = '1 == 1'b1;",
        "\"%h\", U",
        "01",
    );
}

/// ⚠️ Review: the tier-3 count walk must still SEE a fill as sized. `{('1)+1{8'hA5}}`
/// is a zero replication in both oracles (the fill is 32 bits beside the unsized 1,
/// so the sum wraps) and was loud; a fill self width of zero read as "unknown" once
/// sent the count to the engine's lowering, which replicated twice. Loud, on both.
#[test]
fn a_fill_in_a_replication_count_stays_loud_where_the_oracles_reject() {
    loud("  wire [15:0] q = {('1)+1{8'hA5}};", "\"%h\", q");
    loud("  wire [15:0] q = {('1 + '1){8'hA5}};", "\"%h\", q");
}

/// Controls that were right and must stay right: a fill in a DECLARED 8-bit context.
#[test]
fn a_fill_in_a_declared_context_is_unchanged() {
    for (init, want) in [
        ("'1", "ff"),
        ("'0", "00"),
        ("'1 ^ 1'b0", "ff"),
        ("'1 & 4'hF", "0f"),
        ("'1 + 1'b1", "00"),
        ("4'd8 - '1", "09"),
        ("1'b1 ? '1 : 1'b0", "ff"),
        ("8'hFF >> '1", "7f"),
    ] {
        prints(
            &format!("  localparam logic [7:0] U = {init};"),
            "\"%h\", U",
            want,
        );
    }
}
