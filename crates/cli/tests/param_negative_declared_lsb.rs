//! A parameter declared with a NEGATIVE low bound — ROADMAP §2 🆕 L ⓩ residue.
//!
//! `localparam logic [3:-4] A = 8'h3C; localparam L = {A[3:0], A[3:0]};` printed
//! `cc` at exit 0 where both oracles print `33`, and the bare select `A[3:0]` and
//! the bit-select `A[-1]` were honest-loud ("no constant-fold arm"). One root:
//! `DeclRange`'s `lo` was a `u32`, so `param_decl_range_opt` DECLINED a negative
//! bound outright and every consumer fell back to reading the normalised `[w-1:0]`
//! storage positionally. Widening `lo` to `i64` records the declaration truthfully;
//! the keys that gain an entry are exactly the ones with a negative bound.
//!
//! Values are pinned to iverilog 13.0 (`-g2012`) and verilator 5.052
//! (`--binary --timing`), which agree on every cell here.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("vita_pndl_{}_{n}.sv", std::process::id()));
    std::fs::write(&path, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(&path)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_file(&path);
    let so = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "expected exit 0\nstdout:\n{so}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
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

/// The headline cell, its zero-LSB control, and the reads that were loud.
#[test]
fn every_read_of_a_negative_lsb_parameter() {
    let out = run("module top;\n\
           localparam logic [3:-4] A = 8'h3C;\n\
           localparam logic [7:0]  Z = 8'h3C;\n\
           localparam LA = {A[3:0], A[3:0]};\n\
           localparam LZ = {Z[3:0], Z[3:0]};\n\
           localparam SA = A[3:0];\n\
           localparam BA = A[-1];\n\
           localparam WA = {A, A};\n\
           initial begin\n\
             $display(\"LA=%h LZ=%h SA=%h BA=%h WA=%h b=%0d\", LA, LZ, SA, BA, WA, $bits(A));\n\
             $finish;\n\
           end\n\
         endmodule\n");
    // Both oracles. `A[3:0]` names the TOP half of a `[3:-4]` declaration, so it is
    // `3`, not the `c` the same text selects from a `[7:0]` object — LZ is the
    // control that must not move. WA and $bits were already right: a whole-name read
    // owns its declared WIDTH, not its declared position.
    assert_eq!(out, "LA=33 LZ=cc SA=3 BA=1 WA=3c3c b=8\n");
}

/// Both bounds negative, and an ASCENDING negative declaration.
#[test]
fn both_bounds_negative_and_the_ascending_spelling() {
    let out = run("module top;\n\
           localparam logic [-1:-4] A = 4'hC;\n\
           localparam logic [-4:3]  B = 8'h3C;\n\
           initial begin $display(\"A=%h a1=%h b=%0d\", A[-1], A[-1:-4], $bits(B)); $finish; end\n\
         endmodule\n");
    // iverilog: A=1 a1=c b=8. (verilator refuses the ascending declaration itself —
    // `%Warning-ASCRANGE: left < right of bit range` — so `B` is iverilog-only; the
    // `[-1:-4]` cells are two-oracle.)
    assert_eq!(out, "A=1 a1=c b=8\n");
}

/// The package binder and the scoped spelling take the same declared range as the
/// module one — all three maps that carry it move together.
#[test]
fn the_package_binders_carry_the_negative_lsb_too() {
    let out = run("package p;\n\
           parameter logic [3:-4] A = 8'h3C;\n\
         endpackage\n\
         module top;\n\
           import p::*;\n\
           localparam L1 = {A[3:0], A[3:0]};\n\
           localparam L2 = {p::A[3:0], p::A[3:0]};\n\
           initial begin $display(\"L1=%h L2=%h\", L1, L2); $finish; end\n\
         endmodule\n");
    // Both oracles: L1=33 L2=33 — the bare-imported and the scoped spelling of one
    // declaration must not answer differently.
    assert_eq!(out, "L1=33 L2=33\n");
}
