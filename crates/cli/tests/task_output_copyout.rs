//! Task/void-function OUTPUT formal copy-out (IEEE 1800 §13.5.1 / IEEE 1364
//! §10.3.3). An `output`/`inout` formal is a local of the formal's type; an
//! `output` is initialized to that type's default (X for 4-state, 0 for 2-state)
//! at task ENTRY — NOT the caller's prior value — and copied out at return. So a
//! task that does NOT assign an output formal must leave the caller variable at
//! the default, and a read-before-write of an output formal reads the default.
//!
//! The prior inline-task path ALIASED the output formal directly to the caller
//! net (no entry-init), so an unassigned output silently kept the caller's stale
//! value (a silent-wrong). These tests use PLAIN (non-`automatic`) tasks, which
//! take the inline path (recursive/automatic tasks use the already-correct
//! frame-call path). iverilog agrees (output formal = X/0 default).
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> String {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_taskout_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

// ── unassigned 4-state output → caller gets X (the default), not the stale value ──
#[test]
fn unassigned_4state_output_is_x() {
    let o = run("module top;\n\
         reg [7:0] cv;\n\
         task set_if(input c, output [7:0] o);\n\
           if (c) o = 8'hAA;\n\
         endtask\n\
         initial begin\n\
           cv = 8'h55;\n\
           set_if(1'b0, cv);\n\
           $display(\"cv=%b\", cv);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(
        o.contains("cv=xxxxxxxx"),
        "unassigned output must copy out X, not the caller's 8'h55:\n{o}"
    );
}

// ── unassigned 2-state output (int) → caller gets 0 (the default) ──
#[test]
fn unassigned_2state_output_is_zero() {
    let o = run("module top;\n\
         int cv;\n\
         task set_if(input c, output int o);\n\
           if (c) o = 42;\n\
         endtask\n\
         initial begin\n\
           cv = 999;\n\
           set_if(1'b0, cv);\n\
           $display(\"cv=%0d\", cv);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(
        o.contains("cv=0"),
        "unassigned 2-state output must copy out 0, not 999:\n{o}"
    );
}

// ── read-before-write of an output formal reads the default (X), not the caller's ──
#[test]
fn read_before_write_output_reads_default() {
    let o = run("module top;\n\
         reg [7:0] cv;\n\
         task accum(input [7:0] a, output [7:0] o);\n\
           o = o + a;\n\
         endtask\n\
         initial begin\n\
           cv = 8'h05;\n\
           accum(8'h03, cv);\n\
           $display(\"cv=%b\", cv);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    // o reads its X default → X + 3 = X (NOT the caller's 5 + 3 = 8).
    assert!(
        o.contains("cv=xxxxxxxx"),
        "read-before-write output reads X default, so cv must be X (not 8):\n{o}"
    );
}

// ── ASSIGNED output still writes through correctly (regression) ──
#[test]
fn assigned_output_writes_value() {
    let o = run("module top;\n\
         reg [7:0] cv;\n\
         task set_if(input c, output [7:0] o);\n\
           if (c) o = 8'hAA;\n\
         endtask\n\
         initial begin\n\
           cv = 8'h55;\n\
           set_if(1'b1, cv);\n\
           $display(\"cv=%h\", cv);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(
        o.to_lowercase().contains("cv=aa"),
        "assigned output must write 0xAA:\n{o}"
    );
}

// ── §13.4.1: a STATIC task's output formal is a single instance — it retains its
//    value across calls (here across two DIFFERENT call sites) ──
#[test]
fn static_task_output_retains_across_call_sites() {
    let o = run("module top;\n\
         reg [7:0] x, y;\n\
         task t(input set, output [7:0] o);\n\
           if (set) o = 8'hCC;\n\
         endtask\n\
         initial begin\n\
           x = 8'h11; y = 8'h22;\n\
           t(1'b1, x);\n\
           t(1'b0, y);\n\
           $display(\"x=%h y=%h\", x, y);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    // call 1 writes CC → x=CC; call 2 leaves o unwritten → the STATIC formal retains
    // CC → y=CC (not the caller's stale 0x22, not X).
    assert!(
        o.to_lowercase().contains("x=cc") && o.to_lowercase().contains("y=cc"),
        "static output formal retains across call sites (x=cc y=cc):\n{o}"
    );
}

// ── 2-state formal (int) coerces X/Z → 0 on copy-in (§6.11.3) ──
#[test]
fn two_state_input_formal_coerces_xz() {
    let o = run("module top;\n\
         reg [7:0] r;\n\
         task f(input int u);\n\
           r = u[7:0];\n\
         endtask\n\
         initial begin\n\
           f(8'bxxxx_0101);\n\
           $display(\"r=%h\", r);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    // the int formal cannot hold X: the upper nibble coerces to 0 → r = 0x05.
    assert!(
        o.to_lowercase().contains("r=05"),
        "2-state input formal must coerce X→0 (0x05), not keep X:\n{o}"
    );
}

// ── 2-state output formal (int) unassigned → 0 default (not X) on copy-out ──
#[test]
fn two_state_output_formal_default_is_zero() {
    let o = run("module top;\n\
         reg [31:0] r;\n\
         task f(input c, output int o);\n\
           if (c) o = 32'hxxxx_xxxx;\n\
         endtask\n\
         initial begin\n\
           r = 32'h12345678;\n\
           f(1'b1, r);\n\
           $display(\"r=%h\", r);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    // assigning X to a 2-state output coerces to 0 → r = 0.
    assert!(
        o.to_lowercase().contains("r=00000000"),
        "writing X to a 2-state output coerces to 0:\n{o}"
    );
}

// ── nested task threads an OUTER output formal through to an inner task ──
#[test]
fn nested_output_formal_threads_through() {
    let o = run("module top;\n\
         reg [7:0] r;\n\
         task inner(output [7:0] o);\n\
           o = 8'h7E;\n\
         endtask\n\
         task outer(output [7:0] o);\n\
           inner(o);\n\
         endtask\n\
         initial begin\n\
           r = 0;\n\
           outer(r);\n\
           $display(\"r=%h\", r);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    assert!(
        o.to_lowercase().contains("r=7e"),
        "nested output threading must carry 0x7E to the outer caller:\n{o}"
    );
}

// ── narrow INPUT formal truncates the actual to the formal width (copy-in) ──
#[test]
fn narrow_input_truncates_to_formal_width() {
    let o = run("module top;\n\
         reg [7:0] r;\n\
         task f(input [3:0] x);\n\
           r = x;\n\
         endtask\n\
         initial begin\n\
           f(8'hAB);\n\
           $display(\"r=%h\", r);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    // 8'hAB truncates to the 4-bit formal = 0xB, then zero-extends to r = 0x0B.
    assert!(
        o.to_lowercase().contains("r=0b"),
        "narrow input must truncate to the formal width (0x0B), not keep 0xAB:\n{o}"
    );
}

// ── inout formal copies IN then OUT: unassigned inout keeps the caller value ──
#[test]
fn inout_unassigned_keeps_value() {
    let o = run("module top;\n\
         reg [7:0] cv;\n\
         task touch(input c, inout [7:0] o);\n\
           if (c) o = 8'hAA;\n\
         endtask\n\
         initial begin\n\
           cv = 8'h55;\n\
           touch(1'b0, cv);\n\
           $display(\"cv=%h\", cv);\n\
           $finish;\n\
         end\n\
         endmodule\n");
    // inout copies the caller value IN, so an unassigned inout copies the SAME
    // value back OUT — cv stays 0x55 (NOT X).
    assert!(
        o.to_lowercase().contains("cv=55"),
        "unassigned inout keeps the caller value 0x55:\n{o}"
    );
}
