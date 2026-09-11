//! Two same-named INITIALIZER-FREE sibling block-locals inside ONE SUBROUTINE body
//! are two variables (§2 Scoping, ROADMAP §5.2 row 2).
//!
//! ## What was wrong
//!
//! §4.5.480 gave a subroutine body's block-locals the same `$blk$<lo>` scoping a
//! module process's get, but only under the four admission rules
//! `frames_reserve.rs::gather_auto_block_locals` already had: `automatic`, DYNAMIC
//! storage, SHADOWS a module name, and a STATIC declarator carrying an INITIALIZER.
//! A plain `int x;` in a sibling block is none of the four, so the name reached
//! `block_local_class.rs`'s two-span bar with ONE declaring span instead of two, no
//! `$blk$` scope was created, and both declarators coalesced onto one net at
//! `frames_blocal.rs` (`let fq = self.fq(&decl.name.name); if let Some(&existing) =
//! self.symbols.get(&fq) { … continue; }`). The second block read the first's
//! leftover value:
//!
//! ```text
//! task t;
//!   begin int x = 44; $display("A=%0d", x); end
//!   begin int x;      $display("B=%0d", x); end
//! endtask
//! ```
//!
//! printed `A=44 B=44`, where both oracles print `A=44 B=0`.
//!
//! ## The fix
//!
//! A fifth `AdmitReason`, `static_plain` — a STATIC (`d.lifetime != Some(true)`)
//! declarator carrying NO initializer, decl-ANY like `static_init`. It is OPT-IN via
//! `gather_auto_block_locals`'s `admit_static_plain` parameter and is passed `true`
//! ONLY by `compute_scoped_block_locals`'s `for_each_subroutine_body` feed; every
//! module-process feed passes `false`, because that path already answers the same
//! shape LOUDLY (the R18-X1 read-before-assign E3009) and admitting it there would be
//! an unmeasured loud → value move. `block_local_class.rs`'s homogeneity exemption
//! `static_init_only` is widened to `static_only` (every span is `static_init` OR
//! `static_plain`), because the reported pair is MIXED — one init-bearing span beside
//! one initializer-free one.
//!
//! The NESTING filter deliberately keeps `static_init_only` and is NOT widened; see
//! `a_nested_not_sibling_plain_block_local_stays_loud` for the measurement.
//!
//! ## Oracles
//!
//! Every value here was measured three-way against iverilog 13 (`-g2012` + `vvp -n`)
//! and verilator 5.052 (`--binary --timing`), both identical, EXCEPT:
//!
//! * the 4-STATE default (`logic`/`reg`/`integer`): iverilog prints `x`, verilator
//!   prints `0`. Verilator is not an oracle for x/z, so iverilog decides and vita
//!   matches it — `B=x xxxxxxxx` for `logic [7:0]`. Both oracles agree on the thing
//!   the row is about: the read is NOT the sibling's `44`.
//! * `a_mixed_automatic_and_plain_sibling_pair` is 1-oracle (verilator only) —
//!   iverilog 13 rejects `begin automatic int x;` with
//!   `sorry: Overriding the default variable lifetime is not yet supported.`

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_spbl_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).expect("scratch dir");
    std::fs::write(d.join("t.sv"), src).expect("write design");
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

/// A clean run (exit 0) whose output contains every `want` line, verbatim as observed.
fn lines(src: &str, want: &[&str]) {
    let (o, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}");
    }
}

/// A clean run whose output contains `want` and does NOT contain any of `absent`.
fn lines_without(src: &str, want: &[&str], absent: &[&str]) {
    let (o, code) = run(src);
    assert_eq!(code, Some(0), "expected a clean run:\n{o}");
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}");
    }
    for a in absent {
        assert!(!o.contains(a), "did NOT expect {a:?} in:\n{o}");
    }
}

/// A LOUD run (exit 1) whose output carries the given diagnostic CODE.
///
/// Pinned on the code, never on the message TEXT — a wording pin outlives the
/// limitation it describes and breaks on an unrelated rewording.
fn loud_code(src: &str, code_str: &str) {
    let (o, code) = run(src);
    assert_eq!(code, Some(1), "expected a loud run:\n{o}");
    assert!(o.contains(code_str), "expected {code_str:?} in:\n{o}");
}

// ── (1) the reported row, on every route ────────────────────────────────────

/// The row's own design on the INLINE route (a plain `task t;`, `run.json`
/// `"route": "inlined"`). Both oracles: `A=44` / `B=0`.
#[test]
fn plain_sibling_block_local_on_the_inline_route() {
    lines_without(
        r#"module top;
  task t;
    begin int x = 44; $display("A=%0d", x); end
    begin int x; $display("B=%0d", x); end
  endtask
  initial begin t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["A=44", "B=0"],
        &["B=44"],
    );
}

/// The same body on the FRAME route (`task automatic` forces it). Both oracles:
/// `A=44` / `B=0`. The `automatic` keyword is on the TASK, not on the declarators,
/// so both spans are still static and both are `static_plain`/`static_init`.
#[test]
fn plain_sibling_block_local_on_the_frame_route() {
    lines_without(
        r#"module top;
  task automatic t;
    begin int x = 44; $display("A=%0d", x); end
    begin int x; $display("B=%0d", x); end
  endtask
  initial begin t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["A=44", "B=0"],
        &["B=44"],
    );
}

/// A leading `#1;` in a plain `task` body leaves `run.json`'s `"route"` at
/// `"inlined"` — measured, not assumed. Both oracles: `A=44` / `B=0`.
#[test]
fn plain_sibling_block_local_with_a_delay_in_the_body() {
    lines_without(
        r#"module top;
  task t;
    #1;
    begin int x = 44; $display("A=%0d", x); end
    begin int x; $display("B=%0d", x); end
  endtask
  initial begin t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["A=44", "B=0"],
        &["B=44"],
    );
}

/// A HIERARCHICALLY called STATIC task (`u.t()`) — the third route to the same
/// body, a frame over a static task. Both oracles: `A=44` / `B=0`.
#[test]
fn plain_sibling_block_local_in_a_hierarchically_called_static_task() {
    lines_without(
        r#"module sub;
  task t;
    begin int x = 44; $display("A=%0d", x); end
    begin int x; $display("B=%0d", x); end
  endtask
endmodule
module top;
  sub u();
  initial begin u.t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["A=44", "B=0"],
        &["B=44"],
    );
}

/// A FUNCTION body, not a task. Both oracles: `A=44` / `B=0`.
#[test]
fn plain_sibling_block_local_in_a_function_body() {
    lines_without(
        r#"module top;
  function int f;
    begin int x = 44; $display("A=%0d", x); end
    begin int x; $display("B=%0d", x); end
    f = 0;
  endfunction
  int r;
  initial begin r = f(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["A=44", "B=0"],
        &["B=44"],
    );
}

// ── (2) the type axis: what an unassigned read of the fresh net gives ───────

/// `int` (2-state, ≤32). Both oracles `B=0`.
#[test]
fn unassigned_read_of_a_plain_int_sibling_is_zero() {
    lines_without(
        r#"module top;
  task t;
    begin int x = 44; $display("A=%0d %b", x, x); end
    begin int x; $display("B=%0d %b", x, x); end
  endtask
  initial begin t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &[
            "A=44 00000000000000000000000000101100",
            "B=0 00000000000000000000000000000000",
        ],
        &["B=44"],
    );
}

/// `byte` (2-state, ≤32). Both oracles `B=0 00000000`.
#[test]
fn unassigned_read_of_a_plain_byte_sibling_is_zero() {
    lines_without(
        r#"module top;
  task t;
    begin byte x = 8'd44; $display("A=%0d %b", x, x); end
    begin byte x; $display("B=%0d %b", x, x); end
  endtask
  initial begin t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["A=44 00101100", "B=0 00000000"],
        &["B=44"],
    );
}

/// A scalar `bit` (2-state, width 1). Both oracles `B=0 0`.
#[test]
fn unassigned_read_of_a_plain_scalar_bit_sibling_is_zero() {
    lines_without(
        r#"module top;
  task t;
    begin bit x = 1'b1; $display("A=%0d %b", x, x); end
    begin bit x; $display("B=%0d %b", x, x); end
  endtask
  initial begin t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["A=1 1", "B=0 0"],
        &["B=1 1"],
    );
}

/// `bit [7:0]` (2-state, packed range). Both oracles `B=0 00000000`.
#[test]
fn unassigned_read_of_a_plain_packed_bit_sibling_is_zero() {
    lines_without(
        r#"module top;
  task t;
    begin bit [7:0] x = 8'd44; $display("A=%0d %b", x, x); end
    begin bit [7:0] x; $display("B=%0d %b", x, x); end
  endtask
  initial begin t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["A=44 00101100", "B=0 00000000"],
        &["B=44"],
    );
}

/// `logic [7:0]` (4-STATE). vita gives `x`, which is what iverilog 13 gives
/// (`B=x xxxxxxxx`). verilator prints `B=0 00000000` and is NOT an oracle for x/z,
/// so iverilog decides. Both oracles agree it is not the sibling's `44`.
#[test]
fn unassigned_read_of_a_plain_four_state_sibling_is_x() {
    lines_without(
        r#"module top;
  task t;
    begin logic [7:0] x = 8'd44; $display("A=%0d %b", x, x); end
    begin logic [7:0] x; $display("B=%0d %b", x, x); end
  endtask
  initial begin t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["A=44 00101100", "B=x xxxxxxxx"],
        &["B=44"],
    );
}

// ── (3) static RETENTION: each sibling is its own static variable ───────────

/// The soundness question. A STATIC task, TWO initializer-free sibling blocks, called
/// twice. Both oracles: `P=1 Q=10` then `P=2 Q=20` — each sibling RETAINS
/// independently. The fresh `$blk$` net must therefore be STATIC, never per-entry.
/// vita gave `P=1 Q=11 P=12 Q=22` before the fix (one shared net).
#[test]
fn each_plain_sibling_retains_independently_across_calls() {
    lines_without(
        r#"module top;
  task t;
    begin int x; x = x + 1; $display("P=%0d", x); end
    begin int x; x = x + 10; $display("Q=%0d", x); end
  endtask
  initial begin t(); t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["P=1", "Q=10", "P=2", "Q=20"],
        &["Q=11", "P=12", "Q=22"],
    );
}

/// THREE siblings: one init-bearing, one read unassigned, one written then read.
/// Only the middle one was wrong before the fix. Both oracles: `A=44 B=0 C=9`.
#[test]
fn three_plain_siblings_each_get_their_own_net() {
    lines_without(
        r#"module top;
  task t;
    begin int x = 44; $display("A=%0d", x); end
    begin int x; $display("B=%0d", x); end
    begin int x; x = 9; $display("C=%0d", x); end
  endtask
  initial begin t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["A=44", "B=0", "C=9"],
        &["B=44"],
    );
}

/// 1-ORACLE (verilator only): an `automatic` span beside a plain static one. The
/// reason set is MIXED, so `static_only` is false and the pre-existing filter-A
/// nesting test decides — both spans are disjoint, so both survive and both are
/// scoped. verilator prints `P=5 Q=0 P=5 Q=0`; iverilog 13 rejects the design with
/// `sorry: Overriding the default variable lifetime is not yet supported.` Recorded
/// rather than special-cased.
#[test]
fn a_mixed_automatic_and_plain_sibling_pair() {
    lines_without(
        r#"module top;
  task t;
    begin automatic int x; x = 5; $display("P=%0d", x); end
    begin int x; $display("Q=%0d", x); end
  endtask
  initial begin t(); t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["P=5", "Q=0"],
        &["Q=5"],
    );
}

// ── (4) controls that must NOT move ────────────────────────────────────────

/// The control twin: the second block declares `y`, not `x`. Nothing to coalesce,
/// nothing to scope. Three-tool identical before and after: `A=44` / `B=0`.
#[test]
fn control_distinct_names_are_unchanged() {
    lines(
        r#"module top;
  task t;
    begin int x = 44; $display("A=%0d", x); end
    begin int y; $display("B=%0d", y); end
  endtask
  initial begin t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["A=44", "B=0"],
    );
}

/// A SOLO initializer-free block-local (no sibling) keeps today's behaviour: it has
/// ONE declaring span, still falls below `block_local_class.rs`'s two-span bar, and
/// still takes the flatten. Three-tool identical: `S=0 S2=7 S=7 S2=7` — the second
/// call observes the first call's `7`, because a static block-local retains.
#[test]
fn control_a_solo_plain_block_local_is_unchanged() {
    lines(
        r#"module top;
  task t;
    begin int x; $display("S=%0d", x); x = 7; $display("S2=%0d", x); end
  endtask
  initial begin t(); t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["S=0", "S2=7", "S=7", "S2=7"],
    );
}

/// A SOLO block-local in a STATIC task retains across calls: `R=1` then `R=2`.
/// Three-tool identical, and the fix must not turn it into per-entry storage.
#[test]
fn control_a_static_task_local_still_retains() {
    lines_without(
        r#"module top;
  task t;
    begin int x; x = x + 1; $display("R=%0d", x); end
  endtask
  initial begin t(); t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["R=1", "R=2"],
        &["R=3"],
    );
}

/// The `task automatic` twin of the above does NOT retain: `R=1` then `R=1`.
/// Three-tool identical.
#[test]
fn control_an_automatic_task_local_still_does_not_retain() {
    lines_without(
        r#"module top;
  task automatic t;
    begin int x; x = x + 1; $display("R=%0d", x); end
  endtask
  initial begin t(); t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["R=1"],
        &["R=2"],
    );
}

/// The §4.5.480 shape — two siblings BOTH carrying an initializer, already
/// `$blk$`-scoped before this slice. Widening the homogeneity exemption from
/// `static_init_only` to `static_only` must leave it byte-identical:
/// `P=1 Q=101 P=2 Q=102`, three-tool identical.
#[test]
fn control_two_initialised_siblings_are_unchanged() {
    lines_without(
        r#"module top;
  task t;
    begin int x = 0; x = x + 1; $display("P=%0d", x); end
    begin int x = 100; x = x + 1; $display("Q=%0d", x); end
  endtask
  initial begin t(); t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        &["P=1", "Q=101", "P=2", "Q=102"],
        &["P=3", "Q=103"],
    );
}

/// The MODULE-PROCESS twin of the row's design stays LOUD. `admit_static_plain` is
/// `false` for the `for_each_proc` feed exactly so this R18-X1 read-before-assign
/// E3009 keeps firing — the module-process path answers the shape loudly and a loud
/// is a higher rung than a value. Pinned on the CODE, not the message text.
#[test]
fn module_process_plain_sibling_pair_stays_loud() {
    loud_code(
        r#"module top;
  initial begin
    begin int x = 44; $display("A=%0d", x); end
    begin int x; $display("B=%0d", x); end
  end
  initial #100 $finish;
endmodule
"#,
        "VITA-E3009",
    );
}

/// The module-process RETENTION twin stays LOUD for the same reason.
#[test]
fn module_process_plain_retention_pair_stays_loud() {
    loud_code(
        r#"module top;
  initial begin
    begin int x; x = x + 1; $display("P=%0d", x); end
    begin int x; x = x + 10; $display("Q=%0d", x); end
  end
  initial #100 $finish;
endmodule
"#,
        "VITA-E3009",
    );
}

/// A NESTED (not sibling) pair stays LOUD. This is why the nesting filter in
/// `compute_scoped_block_locals` deliberately keeps `static_init_only` and is NOT
/// widened to `static_only`: widening it would scope the outer/inner pair and turn
/// this E3009 into a value. Both oracles print `OUT=44 IN=0`, so the shape is a
/// real capability gap — but a loud → value move belongs to its own measured row,
/// not to this one.
#[test]
fn a_nested_not_sibling_plain_block_local_stays_loud() {
    loud_code(
        r#"module top;
  task t;
    begin
      int x = 44;
      $display("OUT=%0d", x);
      begin
        int x;
        $display("IN=%0d", x);
      end
    end
  endtask
  initial begin t(); #1 $finish; end
  initial #100 $finish;
endmodule
"#,
        "VITA-E3009",
    );
}
