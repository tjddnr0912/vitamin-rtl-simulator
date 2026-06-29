//! Procedural `@(posedge)`/`@(negedge)` use the WIDE 4-state edge definition
//! (ROADMAP §4.5.5 narrow-posedge finding).
//!
//! IEEE 1364 §5.1.2: a posedge is a transition TOWARD 1 — `0→1`, `0→x`, `0→z`,
//! `x→1`, `z→1`; a negedge a transition TOWARD 0 — `1→0`, `1→x`, `1→z`, `x→0`,
//! `z→0`. vita previously used the NARROW form (`new==1 && prev!=1`), which fired
//! posedge only when the LSB actually reached 1 — silently missing `0→x`/`0→z`
//! rises (and `1→x`/`1→z` falls for negedge). That is a silent-wrong for any
//! edge-sensitive net that goes to an UNKNOWN value mid-simulation.
//!
//! Fix (engine only, IR-0): widen `fs_is_posedge`/`fs_is_negedge` (the single
//! source feeding the `slot_edge` edge mask). A net that only ever holds 0/1
//! (every ordinary clock) is unaffected — the new cases require a 0/1→x/z
//! transition — so normal designs are byte-identical. Each case below resets the
//! counters after the t0 settle to ISOLATE the transition under test, pinned to
//! live iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_wideedge_{}_{n}", std::process::id()));
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

/// Drive `a` from `init` to `to` (counters re-zeroed in between) and assert the
/// isolated transition's posedge/negedge counts.
fn edge_counts(init: &str, to: &str) -> String {
    let src = format!(
        "module top; reg a; integer pe, ne;\n\
         always @(posedge a) pe = pe + 1;\n\
         always @(negedge a) ne = ne + 1;\n\
         initial begin pe=0; ne=0; a={init}; #1; pe=0; ne=0; a={to}; #1; $display(\"pe=%0d ne=%0d\", pe, ne); $finish; end\n\
         endmodule\n"
    );
    let (out, code) = run(&src);
    assert_eq!(code, Some(0), "vita exited non-zero: {out}");
    out
}

#[test]
fn zero_to_x_is_posedge() {
    assert!(
        edge_counts("1'b0", "1'bx").contains("pe=1 ne=0"),
        "0→x must be a posedge"
    );
}

#[test]
fn zero_to_z_is_posedge() {
    assert!(
        edge_counts("1'b0", "1'bz").contains("pe=1 ne=0"),
        "0→z must be a posedge"
    );
}

#[test]
fn one_to_x_is_negedge() {
    assert!(
        edge_counts("1'b1", "1'bx").contains("pe=0 ne=1"),
        "1→x must be a negedge"
    );
}

#[test]
fn one_to_z_is_negedge() {
    assert!(
        edge_counts("1'b1", "1'bz").contains("pe=0 ne=1"),
        "1→z must be a negedge"
    );
}

#[test]
fn x_to_one_is_posedge() {
    // Already correct under narrow (new==1); byte-identity guard.
    assert!(
        edge_counts("1'bx", "1'b1").contains("pe=1 ne=0"),
        "x→1 must be a posedge"
    );
}

#[test]
fn x_to_zero_is_negedge() {
    assert!(
        edge_counts("1'bx", "1'b0").contains("pe=0 ne=1"),
        "x→0 must be a negedge"
    );
}

#[test]
fn x_to_z_is_neither() {
    assert!(
        edge_counts("1'bx", "1'bz").contains("pe=0 ne=0"),
        "x→z is neither edge"
    );
}

#[test]
fn z_to_one_is_posedge() {
    assert!(
        edge_counts("1'bz", "1'b1").contains("pe=1 ne=0"),
        "z→1 must be a posedge"
    );
}

#[test]
fn z_to_zero_is_negedge() {
    assert!(
        edge_counts("1'bz", "1'b0").contains("pe=0 ne=1"),
        "z→0 must be a negedge"
    );
}

#[test]
fn z_to_x_is_neither() {
    assert!(
        edge_counts("1'bz", "1'bx").contains("pe=0 ne=0"),
        "z→x is neither edge"
    );
}

#[test]
fn clean_zero_one_byte_identity() {
    // Ordinary 0↔1 clock edges — unchanged by the widening.
    assert!(
        edge_counts("1'b0", "1'b1").contains("pe=1 ne=0"),
        "0→1 posedge"
    );
    assert!(
        edge_counts("1'b1", "1'b0").contains("pe=0 ne=1"),
        "1→0 negedge"
    );
}
