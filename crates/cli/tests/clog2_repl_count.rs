//! `$clog2(k)` of a SINGLE constant argument used as a replication count (or a
//! runtime part-select width) silently folded to a 0-width replication:
//! `{$clog2(8){2'b10}}` printed `00000000` where iverilog gives `00101010`
//! (clog2(8)=3 → {3{10}}). The engine folds a replication count via
//! `const_u32_of_expr`, which reduced only Const/±-of-const; `$clog2` survives
//! lowering as a `SysFunc` node and was never folded. The fix folds `$clog2` of a
//! SINGLE Const argument (its value alone determines clog2 — no width/sign/wrap
//! dependence). An ARITHMETIC argument now folds too, in the elaborate const
//! domain, but only when that domain provably agrees with SV's width-limited
//! arithmetic — see `const_fold_bounds.rs`, which owns that rule. Every value
//! pinned to LIVE iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_clg_{}_{n}", std::process::id()));
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

#[test]
fn clog2_literal_count() {
    // `{$clog2(8){2'b10}}` = {3{10}} = 6'b101010 → 8-bit "00101010".
    let (out, c) = run("module m; logic [7:0] r; initial begin \
         r = {$clog2(8){2'b10}}; $display(\"R=%b\", r); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(out.contains("R=00101010"), "$clog2(8) count; got:\n{out}");
}

#[test]
fn clog2_param_count() {
    // `{$clog2(16){2'b10}}` = {4{10}} = "10101010". A param folds to a Const.
    let (out, c) = run("module m; parameter W = 16; logic [7:0] r; initial begin \
         r = {$clog2(W){2'b10}}; $display(\"R=%b\", r); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("R=10101010"),
        "$clog2(param) count; got:\n{out}"
    );
}

#[test]
fn clog2_edge_and_large_values() {
    // clog2(1)=0 (empty rep inside a concat), clog2(2)=1, clog2(9)=4 (non-power
    // rounds up), clog2(2^25)=25 (large single Const read at full u64, no clamp).
    let (out, c) = run(
        "module m; logic [7:0] p; logic [1:0] q; logic [3:0] r; logic [63:0] s; \
         initial begin \
         p = {4'hF, {$clog2(1){1'b1}}, 4'hA}; q = {$clog2(2){1'b1}}; r = {$clog2(9){1'b1}}; \
         s = {$clog2(32'h2000000){1'b1}}; \
         $display(\"R=%h %b %b %0d\", p, q, r, $countones(s)); #1 $finish; end endmodule\n",
    );
    assert_eq!(c, Some(0));
    assert!(
        out.contains("R=fa 01 1111 25"),
        "clog2 edge + large values; got:\n{out}"
    );
}

#[test]
fn clog2_arithmetic_arg_folds_when_width_exact_else_declines() {
    // An ARITHMETIC `$clog2` argument used to be left entirely to the engine's
    // shallow fold, so BOTH of these were a silent 0-width count. The elaborate
    // const domain now folds the bound/count, but ONLY where its width-unlimited
    // i64 arithmetic provably matches SV's width-limited kind:
    //   a) `N + 1` (N = 255) — every leaf is ≥ 32 bits, so 256 is exact and
    //      `$clog2` is 8, which is what iverilog prints (the old `0` was wrong).
    //   b) `4'd15 + 4'd15` — 4-bit operands WRAP in SV (14, `$clog2` = 4) while
    //      i64 gives 30 (`$clog2` = 5). `const_fold_is_width_exact` declines, so
    //      this keeps the old empty count rather than becoming a WRONG NON-ZERO
    //      one. That decline is the tracked self-width residual (ROADMAP §2).
    let (out, c) = run(
        "module m; parameter N = 255; logic [63:0] a, b; initial begin \
         a = {$clog2(N + 1){1'b1}}; b = {$clog2(4'd15 + 4'd15){1'b1}}; \
         $display(\"R=%0d %0d\", $countones(a), $countones(b)); #1 $finish; end endmodule\n",
    );
    assert_eq!(c, Some(0));
    assert!(
        out.contains("R=8 0"),
        "wide arithmetic clog2 arg folds (8, = iverilog); a narrow one declines to \
         0 rather than a wrong non-zero; got:\n{out}"
    );
}

#[test]
fn clog2_negative_arg_declines() {
    // `$clog2` of a SIGNED-NEGATIVE constant (`4'shF` = -1) is NOT folded (SV
    // widens a negative arg to a ≥32-bit integer). It declines to a 0 count, never
    // a wrong non-zero. (iverilog gives 32; base was also 0, so this is base==fix.)
    let (out, c) = run("module m; logic [63:0] a; initial begin \
         a = {$clog2(4'shF){1'b1}}; $display(\"R=%0d\", $countones(a)); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("R=0"),
        "signed-negative clog2 arg declines to 0; got:\n{out}"
    );
}

#[test]
fn clog2_real_literal_arg_declines() {
    // A REAL literal argument (`$clog2(8.0)`) must NOT be folded — the Const stores
    // the raw f64 bit pattern, not an integer. It declines to a 0 count (base==fix),
    // never a wrong non-zero (a naive read would give clog2(0x4020_0000_0000_0000)).
    let (out, c) = run("module m; logic [255:0] a; initial begin \
         a = {$clog2(8.0){1'b1}}; $display(\"R=%0d\", $countones(a)); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("R=0"),
        "real-literal clog2 arg declines to 0, never a wrong non-zero; got:\n{out}"
    );
}

#[test]
fn clog2_of_runtime_still_loud() {
    // A $clog2 of a RUNTIME variable is non-constant → still loud (the elaborate
    // guard rejects it before the engine).
    let (out, c) = run("module m; int n = 8; logic [7:0] r; initial begin \
         r = {$clog2(n){1'b1}}; $display(\"R=%b\", r); #1 $finish; end endmodule\n");
    assert_ne!(c, Some(0), "$clog2(runtime) must be loud; got:\n{out}");
}

#[test]
fn non_clog2_counts_unchanged() {
    // Byte-identity guard: a param / literal / const-function count is untouched
    // (never routes through the new `$clog2` arm), so output is unchanged.
    let (out, c) = run("module m;\n\
         parameter P = 3;\n\
         function integer f(input integer x); f = x; endfunction\n\
         logic [11:0] a, b, c2;\n\
         initial begin a = {P{4'hA}}; b = {3{4'hA}}; c2 = {f(3){4'hA}};\n\
           $display(\"R=%h %h %h\", a, b, c2); #1 $finish; end endmodule\n");
    assert_eq!(c, Some(0));
    assert!(
        out.contains("R=aaa aaa aaa"),
        "non-clog2 counts unchanged; got:\n{out}"
    );
}
