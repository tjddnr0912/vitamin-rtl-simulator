//! Two drivers on one variable is an ERROR (`VITA-E3001`) — and it is
//! `always_comb` only, which is the half of this file that took measuring.
//!
//! IEEE §9.2.2.2: a variable written by `always_comb` may have no other driver,
//! and a declaration initializer is one. vita had this as a WARNING, so
//! `logic rdy = 1'b1; always_comb rdy = …;` ran at exit 0 with nothing said and
//! then stopped xrun's elaboration (`*E,MULAXX`). The owner's ruling is that a
//! multiple-driver design is not a style question — it now stops the run.
//!
//! ## ⚠️⚠️ Why `always_ff` and `always_latch` are NOT in the set
//!
//! An external report asked for both, on the ground that the clause "does not
//! vary by block kind". It does. Measured, one kind per file, verilator 5.050
//! `--lint-only -Wall`:
//!
//! ```text
//!   always_comb  + initializer -> MULTIDRIVEN (cites IEEE 1800-2023 9.2.2.2)
//!   always_ff    + initializer -> PROCASSINIT only  (a style note)
//!   always_latch + initializer -> PROCASSINIT only
//! ```
//!
//! iverilog says nothing about any of the three. And verilator's split is not an
//! oversight — it is what the rule is FOR. `always_comb` models combinational
//! logic, whose output must be a function of its inputs at all times, so any
//! other write destroys the property the procedure asserts. `always_ff` models a
//! REGISTER, and a declaration initializer is that register's power-on value:
//! `logic [7:0] c = 0; always_ff @(posedge clk) c <= c + 1;` is the ordinary FPGA
//! initialization idiom that synthesis tools implement.
//!
//! ⭐ The widened version was built, and the thing that refuted it was this
//! repository's own `obs_procs` fixture, which is written in exactly that idiom
//! and stopped elaborating. A working test design breaking is evidence AGAINST a
//! new rejection, not for it — it was read the wrong way round for one commit.
//!
//! ## ⚠️ Plain `always` is out for a different reason
//!
//! `logic clk = 1'b0; always #5 clk = ~clk;` is the clock generator every
//! testbench has, and the LRM clause reaches only the inference procedures. So
//! are `initial` and `final`.
//!
//! ## ⚠️ Promotion is the one ladder move that can DESCEND
//!
//! A false positive now rejects legal RTL instead of merely annoying its author.
//! The detector is `stmt_never_writes_ident`, the definite-assignment walk, which
//! over-approximates on purpose (name-based; an unresolved call writes
//! everything) because it was built for accept gates. It had a measured false
//! positive on a block-local SHADOW; `declares_local_named` closes it, and
//! `a_block_local_shadow_is_not_a_second_driver` is that cell.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_mdip_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    let f = d.join("t.sv");
    std::fs::write(&f, src).expect("write design");
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg("t.sv")
        .current_dir(&d)
        .output()
        .expect("run vita");
    let _ = std::fs::remove_dir_all(&d);
    let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    (s, out.status.code())
}

/// How many multi-driver diagnostics the run printed.
fn drivers(out: &str) -> usize {
    out.matches("declaration initializer AND").count()
}

/// The rule itself: elaboration stops, so there is no simulated value to check —
/// the diagnostic IS the answer.
#[test]
fn an_always_comb_write_over_an_initializer_is_rejected() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  logic d = 0;\n  logic cmb = 1'b0;\n  \
         always_comb cmb = ~d;\n  \
         initial begin #1 $display(\"c=%0b\", cmb); $finish; end\nendmodule\n",
    );
    assert_ne!(code, Some(0), "two drivers stop elaboration:\n{out}");
    assert_eq!(drivers(&out), 1, "exactly `cmb`, not `d`:\n{out}");
    assert!(out.contains("VITA-E3001"), "{out}");
    assert!(
        out.contains("`cmb`") && out.contains("`always_comb`"),
        "{out}"
    );
    assert!(out.contains("§9.2.2.2"), "{out}");
}

/// ⚠️⚠️ **The FPGA power-on idiom, and the cell that refuted the widened rule.**
///
/// A register's declaration initializer is its reset value, not a second driver.
/// verilator agrees (PROCASSINIT, a style note — not MULTIDRIVEN), iverilog says
/// nothing, and synthesis tools implement it. This is also the shape
/// `crates/cli/tests/obs_procs.rs`'s hand-checkable fixture is written in, which
/// is how the over-wide version was caught.
#[test]
fn an_always_ff_over_an_initializer_is_accepted() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  logic clk = 0;\n  logic [7:0] c = 0;\n  \
         always #5 clk = ~clk;\n  \
         always_ff @(posedge clk) c <= c + 8'd1;\n  \
         initial begin #100 $display(\"c=%0d\", c); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "a power-on value is not a driver:\n{out}");
    assert_eq!(drivers(&out), 0, "{out}");
    assert!(out.contains("c=10"), "and it really counts from 0:\n{out}");
}

/// `always_latch` sits with `always_ff` here, on the same measurement.
#[test]
fn an_always_latch_over_an_initializer_is_accepted() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  logic en = 0, d = 1;\n  logic lat = 1'b0;\n  \
         always_latch if (en) lat = d;\n  \
         initial begin #1 en = 1; #1 $display(\"lat=%0b\", lat); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(drivers(&out), 0, "{out}");
    assert!(out.contains("lat=1"), "{out}");
}

/// The clock generator every testbench has. If this ever starts firing, the check
/// has stopped being useful and started rejecting working testbenches.
#[test]
fn a_plain_always_over_an_initializer_is_accepted() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  logic clk = 1'b0;\n  logic tog = 1'b0;\n  \
         always #5 clk = ~clk;\n  always @(posedge clk) tog = ~tog;\n  \
         initial begin #100 $display(\"c=%0b t=%0b\", clk, tog); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(drivers(&out), 0, "{out}");
}

/// `initial` writing an initialized variable is ordinary testbench setup.
#[test]
fn an_initial_over_an_initializer_is_accepted() {
    let (out, code) = run("`timescale 1ns/1ns\nmodule tb;\n  int seed = 7;\n  \
         initial begin seed = 9; #1 $display(\"s=%0d\", seed); $finish; end\nendmodule\n");
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(drivers(&out), 0, "{out}");
}

/// One driver is not two: `always_comb` over a variable with NO initializer is
/// the whole point of writing one.
#[test]
fn an_always_comb_without_an_initializer_is_accepted() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  logic d = 0;\n  logic cmb;\n  \
         always_comb cmb = ~d;\n  \
         initial begin #1 $display(\"c=%0b\", cmb); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(drivers(&out), 0, "only `d` has an initializer:\n{out}");
}

/// TWO `always_comb` blocks writing the same initialized variable is still ONE
/// diagnostic. The caret is the DECLARATION's either way, so a second copy would
/// repeat itself at the same character.
#[test]
fn two_writers_over_one_initializer_report_once() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  logic d = 0, e = 0;\n  logic cmb = 1'b0;\n  \
         always_comb cmb = ~d;\n  always_comb cmb = ~e;\n  \
         initial begin #1 $display(\"c=%0b\", cmb); $finish; end\nendmodule\n",
    );
    assert_ne!(code, Some(0), "{out}");
    assert_eq!(drivers(&out), 1, "one initializer, one diagnostic:\n{out}");
}

/// The write walk is the conservative one the definite-assignment analysis uses,
/// so a write nested inside a block or a branch counts exactly as a top-level one
/// does. Pinned because a shallower walk would pass every cell above and miss the
/// shape real RTL is written in.
#[test]
fn a_nested_write_inside_an_always_comb_still_counts() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  logic sel = 0, a = 1, b = 0;\n  \
         logic cmb = 1'b0;\n  \
         always_comb begin\n    if (sel) begin\n      cmb = a;\n    \
         end else begin\n      cmb = b;\n    end\n  end\n  \
         initial begin #1 $display(\"c=%0b\", cmb); $finish; end\nendmodule\n",
    );
    assert_ne!(code, Some(0), "{out}");
    assert_eq!(drivers(&out), 1, "the nested write is a driver:\n{out}");
}

/// ⚠️⚠️ **The false positive the promotion had to close first.**
///
/// The module-level `n` is written by NOBODY: the `always_comb` declares its own
/// block-local `n` and writes that. `stmt_never_writes_ident` cannot tell them
/// apart — it is name-based, and it over-approximates writes on purpose because
/// it was built for accept gates, where erring toward "written" is the safe
/// direction. As a warning that was a nuisance; as an error it would reject RTL
/// that iverilog, verilator and xrun all accept.
///
/// ⚠️ Suppressing the DIAGNOSTIC does not make this design right in vita: it
/// prints the block-local's value for the module variable, because v1 flattens a
/// procedural block-local onto a module net by bare name and the two coalesce.
/// That is a pre-existing silent-wrong of its own and is deliberately NOT what
/// this cell asserts — the guard is a fact about the SOURCE (a shadow is
/// declared), not about which net vita happens to use, so the two can be fixed
/// independently.
#[test]
fn a_block_local_shadow_is_not_a_second_driver() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  logic d = 0;\n  int n = 7;\n  int out1;\n  \
         always_comb begin\n    int n;\n    n = 3;\n    out1 = n + int'(d);\n  end\n  \
         initial begin #1 $display(\"done\"); $finish; end\nendmodule\n",
    );
    assert_eq!(code, Some(0), "a shadow is not a driver:\n{out}");
    assert_eq!(drivers(&out), 0, "{out}");
}

/// The guard is about a DECLARED shadow, not about any write vanishing. A write
/// reaching the module variable through a task's `inout` actual is a real driver
/// and still stops the run — this keeps the guard from being a blanket "if
/// anything is complicated, stay quiet".
#[test]
fn a_write_through_a_task_inout_is_still_a_driver() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  logic d = 0;\n  int acc = 0;\n  \
         task automatic bump(inout int v); v = v + 1; endtask\n  \
         always_comb bump(acc);\n  \
         initial begin #1 $display(\"done\"); $finish; end\nendmodule\n",
    );
    assert_ne!(code, Some(0), "{out}");
    assert_eq!(drivers(&out), 1, "{out}");
    assert!(out.contains("`acc`"), "{out}");
}

/// A shadow in ONE procedure does not excuse a genuine driver in another. Pinned
/// because the guard skips the whole procedure that declares the shadow, and the
/// cheap way to write that would have been to skip the whole variable.
#[test]
fn a_shadow_in_one_procedure_does_not_excuse_a_driver_in_another() {
    let (out, code) = run(
        "`timescale 1ns/1ns\nmodule tb;\n  logic d = 0;\n  int n = 7;\n  int out1;\n  \
         always_comb begin\n    int n;\n    n = 3;\n    out1 = n + int'(d);\n  end\n  \
         always_comb n = 1 + int'(d);\n  \
         initial begin #1 $display(\"done\"); $finish; end\nendmodule\n",
    );
    assert_ne!(code, Some(0), "the second block really writes `n`:\n{out}");
    assert_eq!(drivers(&out), 1, "{out}");
    assert!(out.contains("`n`"), "{out}");
}
