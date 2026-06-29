//! A zero-count replication `{0{x}}` has width 0 (IEEE §11.4.12.1) — it is legal
//! only inside a concatenation, where it contributes nothing. vita's self-width
//! table forced a minimum of 1 bit (`.max(1)`), injecting a spurious bit, so
//! `{4'hA,{0{1'b1}},4'h5}` sized to 9 bits and printed `45` instead of `a5` (and a
//! trailing `{0{x}}` was even worse). A silent-wrong, common via the conditional-
//! concat idiom `{COND{...}}` with COND=0. Pinned to iverilog 13.0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_zrep_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn zero_rep_middle_contributes_nothing() {
    // `{4'hA,{0{1'b1}},4'h5}` → "a5" (the zero-rep is empty, NOT a spurious bit).
    let out = run(
        "module top; reg [7:0] a; initial begin a={4'hA,{0{1'b1}},4'h5}; \
         $display(\"[%h]\",a); $finish; end endmodule\n",
    );
    assert!(out.contains("[a5]"), "middle {{0{{}}}}; got:\n{out}");
}

#[test]
fn zero_rep_trailing_contributes_nothing() {
    // `{8'hC3,{0{1'b1}}}` → "c3".
    let out = run(
        "module top; reg [7:0] a; initial begin a={8'hC3,{0{1'b1}}}; \
         $display(\"[%h]\",a); $finish; end endmodule\n",
    );
    assert!(out.contains("[c3]"), "trailing {{0{{}}}}; got:\n{out}");
}

#[test]
fn zero_rep_parameter_conditional_concat() {
    // The conditional-concat idiom `{EN{...}}` with EN=0 drops the field.
    let out = run(
        "module top; parameter EN=0; reg [7:0] a; initial begin a={4'hA,{EN{4'hF}},4'h5}; \
         $display(\"[%h]\",a); $finish; end endmodule\n",
    );
    assert!(out.contains("[a5]"), "EN=0 conditional concat; got:\n{out}");
}

#[test]
fn nonzero_rep_unchanged() {
    // Byte-identity: a nonzero replication is unaffected (`{EN{4'hF}}` EN=1 → "af5").
    let out = run(
        "module top; parameter EN=1; reg [11:0] a; initial begin a={4'hA,{EN{4'hF}},4'h5}; \
         $display(\"[%h]\",a); $finish; end endmodule\n",
    );
    assert!(out.contains("[af5]"), "EN=1 nonzero rep; got:\n{out}");
}

#[test]
fn multiple_zero_reps() {
    // Several `{0{}}` fields all drop: `{4'h3,{0{1'b1}},{0{4'hF}},4'hC}` → "3c".
    let out = run(
        "module top; reg [7:0] a; initial begin a={4'h3,{0{1'b1}},{0{4'hF}},4'hC}; \
         $display(\"[%h]\",a); $finish; end endmodule\n",
    );
    assert!(out.contains("[3c]"), "multiple zero-reps; got:\n{out}");
}
