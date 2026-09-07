//! A SELECT of a parameter whose declared range does not start at 0, folded in
//! the wide (carry-free) bit domain — ROADMAP §2 🆕 L ⓩ.
//!
//! `localparam logic [11:4] A = 8'h3C; localparam L = {A[7:4], A[7:4]};` printed
//! `33` at exit 0 where both oracles print `cc`: the placement folder reads a
//! select POSITIONALLY (`bp_get(&b, lsb + i)`) against the stored `[w-1:0]`
//! value, so `A[7:4]` took the value's top nibble instead of the declared bits
//! 7..4. The same text OUTSIDE a concatenation was already correct — the i64
//! lane answers it through `const_param_select`, which normalizes by the
//! declared range — so vita disagreed with itself two characters apart.
//!
//! Every value below is pinned to iverilog 13.0 (`-g2012`) AND verilator 5.052
//! (`--binary --timing`), which agree on all of them; the zero-LSB twin in each
//! test is the control that was already right and must not move.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn write_src(src: &str) -> std::path::PathBuf {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_pdls_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    path
}

fn run(src: &str) -> String {
    let path = write_src(src);
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "expected exit 0\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    let mut s = String::new();
    for l in so
        .lines()
        .filter(|l| !l.starts_with("simulation ended") && !l.starts_with("errors="))
    {
        s.push_str(l);
        s.push('\n');
    }
    s
}

fn run_loud(src: &str) -> String {
    let path = write_src(src);
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    assert!(
        !out.status.success(),
        "expected a loud reject (nonzero exit), got:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

/// The headline cell, beside its zero-LSB control.
#[test]
fn part_select_in_a_concat_reads_the_declared_range() {
    let out = run("module top;\n\
           localparam logic [11:4] A1 = 8'h3C;\n\
           localparam logic [7:0]  A0 = 8'h3C;\n\
           localparam L1 = {A1[7:4], A1[7:4]};\n\
           localparam L0 = {A0[7:4], A0[7:4]};\n\
           initial begin $display(\"L1=%h L0=%h\", L1, L0); $finish; end\n\
         endmodule\n");
    // iverilog + verilator: L1=cc L0=33. `A1[7:4]` names declared bits 7..4,
    // which are the LOW nibble of the stored byte; `A0[7:4]` is the high one.
    assert_eq!(out, "L1=cc L0=33\n");
}

/// A BIT select goes through the same arm — it already asked the resolver first,
/// but the resolver only answered a constant ARRAY element.
#[test]
fn bit_select_in_a_concat() {
    let out = run("module top;\n\
           localparam logic [11:4] A = 8'h3C;\n\
           localparam L = {A[7], A[7]};\n\
           initial begin $display(\"L=%h bits=%0d\", L, $bits(L)); $finish; end\n\
         endmodule\n");
    // Both oracles: L=3 bits=2 (declared bit 7 is stored bit 3, which is 1).
    assert_eq!(out, "L=3 bits=2\n");
}

/// Both INDEXED spellings of the same four bits.
#[test]
fn indexed_part_selects_in_a_concat() {
    let out = run("module top;\n\
           localparam logic [11:4] A = 8'h3C;\n\
           localparam M = {A[7 -: 4], A[7 -: 4]};\n\
           localparam P = {A[4 +: 4], A[4 +: 4]};\n\
           initial begin $display(\"M=%h P=%h\", M, P); $finish; end\n\
         endmodule\n");
    // Both oracles: M=cc P=cc.
    assert_eq!(out, "M=cc P=cc\n");
}

/// The upper half of the declared range, and the full range. Both were LOUD
/// (E3009) before, because the positional read ran off the stored width.
#[test]
fn upper_half_and_full_declared_range_fold() {
    let out = run("module top;\n\
           localparam logic [11:4] A = 8'h3C;\n\
           localparam U = {A[11:8], A[11:8]};\n\
           localparam F = {A[11:4], A[11:4]};\n\
           initial begin $display(\"U=%h F=%h\", U, F); $finish; end\n\
         endmodule\n");
    // Both oracles: U=33 F=3c3c.
    assert_eq!(out, "U=33 F=3c3c\n");
}

/// An ASCENDING declaration, with and without a shifted low bound. Both were
/// LOUD before; the declared-range fold carries the direction.
#[test]
fn ascending_declarations_fold() {
    let out = run("module top;\n\
           localparam logic [0:7]  A = 8'h3C;\n\
           localparam logic [4:11] B = 8'h3C;\n\
           localparam LA = {A[0:3], A[0:3]};\n\
           localparam LB = {B[4:7], B[4:7]};\n\
           initial begin $display(\"LA=%h LB=%h\", LA, LB); $finish; end\n\
         endmodule\n");
    // Both oracles: LA=33 LB=33 (declared index 0 / 4 is the MSB either way).
    assert_eq!(out, "LA=33 LB=33\n");
}

/// A WHOLE-name read owns its declared WIDTH, not its declared position — the
/// reason the shift may not live on the name resolver.
#[test]
fn whole_name_read_is_unshifted() {
    let out = run("module top;\n\
           localparam logic [11:4] A = 8'h3C;\n\
           localparam W = {A, A};\n\
           initial begin $display(\"W=%h bits=%0d\", W, $bits(W)); $finish; end\n\
         endmodule\n");
    // Both oracles: W=3c3c bits=16 — unchanged, and the cell that fails if the
    // declared LSB is applied to the name instead of to the select.
    assert_eq!(out, "W=3c3c bits=16\n");
}

/// Every binder the fold is reachable through: package, generate scope, an
/// override ACTUAL, an overridden header parameter, `parameter` vs `localparam`,
/// and a `signed` declaration.
#[test]
fn every_binder_folds_the_declared_range() {
    let pkg = run("package pk;\n\
           localparam logic [11:4] A = 8'h3C;\n\
           localparam L = {A[7:4], A[7:4]};\n\
         endpackage\n\
         module top;\n\
           import pk::*;\n\
           initial begin $display(\"L=%h bits=%0d\", L, $bits(L)); $finish; end\n\
         endmodule\n");
    assert_eq!(pkg, "L=cc bits=8\n");

    let gen = run("module top;\n\
           generate if (1) begin : g\n\
             localparam logic [11:4] A = 8'h3C;\n\
             localparam L = {A[7:4], A[7:4]};\n\
             initial begin $display(\"L=%h bits=%0d\", L, $bits(L)); $finish; end\n\
           end endgenerate\n\
         endmodule\n");
    assert_eq!(gen, "L=cc bits=8\n");

    // The override ACTUAL. `$bits` was 32 here as well as the value being wrong:
    // the declared-width consumer takes its answer from the same fold.
    let ovr = run("module sub #(parameter P = 0);\n\
           initial $display(\"P=%h bits=%0d\", P, $bits(P));\n\
         endmodule\n\
         module top;\n\
           localparam logic [11:4] A = 8'h3C;\n\
           sub #(.P({A[7:4], A[7:4]})) u();\n\
           initial #100 $finish;\n\
         endmodule\n");
    assert_eq!(ovr, "P=cc bits=8\n");

    // The select's BASE is itself an overridden header parameter.
    let hdr = run("module sub #(parameter logic [11:4] W = 8'h11);\n\
           localparam L = {W[7:4], W[7:4]};\n\
           initial $display(\"L=%h bits=%0d\", L, $bits(L));\n\
         endmodule\n\
         module top; sub #(.W(8'h3C)) u(); initial #100 $finish; endmodule\n");
    assert_eq!(hdr, "L=cc bits=8\n");

    let param = run("module top;\n\
           parameter logic [11:4] A = 8'h3C;\n\
           localparam L = {A[7:4], A[7:4]};\n\
           initial begin $display(\"L=%h\", L); $finish; end\n\
         endmodule\n");
    assert_eq!(param, "L=cc\n");

    let signed_decl = run("module top;\n\
           localparam logic signed [11:4] A = 8'h3C;\n\
           localparam L = {A[7:4], A[7:4]};\n\
           initial begin $display(\"L=%h\", L); $finish; end\n\
         endmodule\n");
    assert_eq!(signed_decl, "L=cc\n");
}

/// Inside a constant FUNCTION body, where the interpreter carries its own
/// environment — and where vita's own runtime call already answered `cc`.
#[test]
fn const_function_body_folds_the_declared_range() {
    let out = run("module top;\n\
           localparam logic [11:4] A = 8'h3C;\n\
           function automatic [7:0] f(); f = {A[7:4], A[7:4]}; endfunction\n\
           localparam L = f();\n\
           initial begin $display(\"L=%h Lf=%h\", L, f()); $finish; end\n\
         endmodule\n");
    // Both oracles: L=cc Lf=cc. Before, the constant lane said 33 and the
    // runtime call said cc, in one line.
    assert_eq!(out, "L=cc Lf=cc\n");
}

/// A select reaching OUTSIDE the declared range is `x` (§11.5.1), so the untyped
/// integer localparam that holds it is honest-loud — never the bits that happen
/// to sit at those positions in the stored value.
#[test]
fn out_of_declared_range_select_is_loud_not_the_stored_bits() {
    // `A[3:0]` on a `[11:4]` declaration names four bits the object does not
    // have. Both oracles print `xx`; vita printed `cc` (the stored low nibble).
    let err = run_loud(
        "module top;\n\
           localparam logic [11:4] A = 8'h3C;\n\
           localparam L = {A[3:0], A[3:0]};\n\
           initial begin $display(\"L=%h\", L); $finish; end\n\
         endmodule\n",
    );
    assert!(
        err.contains("E-ELAB-UNSUPPORTED"),
        "expected the constant-fold decline, got:\n{err}"
    );

    // The zero-LSB control: the SAME text on `[7:0]` is in range and folds.
    let ok = run("module top;\n\
           localparam logic [7:0] A = 8'h3C;\n\
           localparam L = {A[3:0], A[3:0]};\n\
           initial begin $display(\"L=%h\", L); $finish; end\n\
         endmodule\n");
    assert_eq!(ok, "L=cc\n");
}

/// The bare select — outside any concatenation — is the path that was already
/// right, and the wide arm must not have moved it.
#[test]
fn bare_selects_outside_a_concat_are_unchanged() {
    let out = run("module top;\n\
           localparam logic [11:4] A = 8'h3C;\n\
           localparam S = A[7:4];\n\
           localparam B = A[7];\n\
           localparam M = A[7 -: 4];\n\
           localparam P = A[4 +: 4];\n\
           wire [A[7:4]:0] w;\n\
           initial begin\n\
             $display(\"S=%h B=%h M=%h P=%h wb=%0d\", S, B, M, P, $bits(w));\n\
             $finish;\n\
           end\n\
         endmodule\n");
    // Both oracles: S=c B=1 M=c P=c wb=13.
    assert_eq!(out, "S=c B=1 M=c P=c wb=13\n");
}
