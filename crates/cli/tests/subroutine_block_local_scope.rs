//! Two same-named sibling block-locals inside ONE SUBROUTINE body are two variables
//! (§2 Scoping, ROADMAP §5.2 row 3).
//!
//! ## What was wrong
//!
//! `compute_scoped_block_locals` / `compute_coalesced_block_locals` were computed over
//! `for_each_proc(&module.body, …)`, whose `walk_items` matches
//! `ast::ModuleItem::Proc(p)` and drops task/function declarations at `_ => {}`. So
//! `scoped_block_locals` never held a span from a subroutine body,
//! `block_local_scope_seg` answered `None` for every block inside one, and both
//! subroutine reservers keyed a block-local's storage on the BARE NAME under a single
//! per-subroutine scope segment:
//!
//! * `inline_task.rs::hoist_inline_task_locals` — `let key = self.fq(&decl.name.name);`
//!   under `$itask$<name>$L`. This is the path a plain `task t;`, a `task static t;`
//!   and a `function void f;` take, i.e. the one the reported row is on.
//! * `frames_reserve.rs::reserve_frame_block_locals` — the same line under
//!   `$func$<name>`, for `automatic`, a value-returning function and a hierarchically
//!   called task.
//!
//! On the inline path both declarators' initializers were then emitted against that
//! one net at first call, in source order, so the LAST one won and BOTH readers saw it
//! (`A=55 B=55` where both oracles print `A=44 B=55`).
//!
//! ## The fix
//!
//! `for_each_subroutine_body` feeds the module's task/function bodies (including the
//! ones inside a `generate`) to the SAME gatherer with the SAME `module_names` set, so
//! a subroutine block-local earns a `$blk$<lo>` segment under exactly the admission
//! rules a module-process one does — no new `AdmitReason`. The two reservers take the
//! declaring block's span chain (`collect_block_local_decls_spanned`) and reserve under
//! `block_local_scope_prefix`, which reproduces the segments the Logic-phase
//! `Stmt::Block` arm (`stmt_main.rs`) wraps the block body in; the inline path's
//! one-shot initializer emission is wrapped identically, or the write would land on the
//! new slot and the read on the old.
//!
//! `compute_coalesced_block_locals` deliberately did NOT get the same walk — see
//! `module_process_block_local_beside_a_task_of_the_same_name` below for the measured
//! correct → loud it causes.
//!
//! ## Oracles
//!
//! Every value here was measured three-way against iverilog 13 (`-g2012` + `vvp -n`)
//! and verilator 5.052 (`--binary --timing`), both identical, except where a test says
//! otherwise: the packed-struct shapes are verilator-only (iverilog 13 rejects
//! `st_t x = '{a:44};` inside a `begin…end` with `syntax error` / `error: Syntax error
//! in variable list.`), and the `fork` shape's ORDER is not oracle-decidable (iverilog
//! prints `B=55` first, verilator `A=44` first) so only its two VALUES are pinned.

use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

fn run(src: &str) -> (String, Option<i32>) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_sbls_{}_{n}", std::process::id()));
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

/// A LOUD run (exit 1) whose output contains every `want` fragment.
fn loud(src: &str, want: &[&str]) {
    let (o, code) = run(src);
    assert_eq!(code, Some(1), "expected a loud run:\n{o}");
    for w in want {
        assert!(o.contains(w), "expected {w:?} in:\n{o}");
    }
}

// ── (1) the reported row's own two designs ──────────────────────────────────

/// The ROADMAP §5.2 row 3 design: `o1=44 o2=55`, not `o1=55 o2=55`. Both oracles
/// print `o1=44 o2=55`.
#[test]
fn the_roadmap_row_design_is_two_variables() {
    lines(
        r#"module t;
  int o1, o2;
  task tt;
    begin : b1
      int s = 44;
      o1 = s;
    end
    begin : b2
      int s = 55;
      o2 = s;
    end
  endtask
  initial begin tt; #2 $display("o1=%0d o2=%0d", o1, o2); $finish; end
endmodule
"#,
        &["o1=44 o2=55"],
    );
}

/// The ROADMAP row's four-call ladder. A STATIC task's locals are retained across
/// calls and a static initializer runs ONCE at t0, so each variable increments on its
/// own: `45 56 46 57 47 58 48 59`. vita printed `56 57 58 59 60 61 62 63` — one
/// shared cell incremented eight times. Both oracles print the first form.
#[test]
fn the_roadmap_row_four_call_ladder() {
    lines(
        r#"module top;
  task t;
    begin
      begin : BA int x = 44; x = x + 1; $display("A=%0d", x); end
      begin : BB int x = 55; x = x + 1; $display("B=%0d", x); end
    end
  endtask
  initial begin t(); t(); t(); t(); #1 $finish; end
endmodule
"#,
        &[
            "A=45", "B=56", "A=46", "B=57", "A=47", "B=58", "A=48", "B=59",
        ],
    );
}

// ── (2) the silent → value cells, by subroutine kind ────────────────────────

/// A00 — `task t;` (the default static task), the base shape.
#[test]
fn a_plain_static_task() {
    lines(
        r#"module top;
  task t;
    begin
      begin : BA int x = 44; $display("A=%0d", x); end
      begin : BB int x = 55; $display("B=%0d", x); end
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
    );
}

/// A10 — `task static t;`, the explicit spelling of the same lifetime.
#[test]
fn an_explicit_static_task() {
    lines(
        r#"module top;
  task static t;
    begin
      begin : BA int x = 44; $display("A=%0d", x); end
      begin : BB int x = 55; $display("B=%0d", x); end
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
    );
}

/// A20 — `function void f;`. This is the cell that refutes the row's own control
/// claim ("function is right"): a void function routes `inlined` and was silent-wrong
/// exactly like the task.
#[test]
fn a_void_function() {
    lines(
        r#"module top;
  function void f;
    begin
      begin : BA int x = 44; $display("A=%0d", x); end
      begin : BB int x = 55; $display("B=%0d", x); end
    end
  endfunction
  initial begin f(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
    );
}

/// A30 / A40 — the `automatic` twins were already correct on this shape (their block
/// decl-inits are emitted at BLOCK ENTRY inside the frame body, which masked the
/// shared slot) and must stay correct.
#[test]
fn the_automatic_task_twin_stays_correct() {
    lines(
        r#"module top;
  task automatic t;
    begin
      begin : BA int x = 44; $display("A=%0d", x); end
      begin : BB int x = 55; $display("B=%0d", x); end
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
    );
}

/// A40 — the `function automatic void` twin.
#[test]
fn the_automatic_function_twin_stays_correct() {
    lines(
        r#"module top;
  function automatic void f;
    begin
      begin : BA int x = 44; $display("A=%0d", x); end
      begin : BB int x = 55; $display("B=%0d", x); end
    end
  endfunction
  initial begin f(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
    );
}

// ── (3) the type axis ───────────────────────────────────────────────────────

/// B0t2 — `logic [7:0]`.
#[test]
fn a_vector_pair() {
    lines(
        r#"module top;
  task t;
    begin
      begin : BA logic [7:0] x = 8'd44; $display("A=%0d", x); end
      begin : BB logic [7:0] x = 8'd55; $display("B=%0d", x); end
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
    );
}

/// B0t3 — a fixed UNPACKED array pair. The element-addressing sidecars
/// (`array_dims` / `array_dim_desc` / `unpacked_array_nets`) are keyed off the net the
/// reserve loop creates, so they follow the scope automatically.
#[test]
fn an_unpacked_array_pair() {
    lines(
        r#"module top;
  task t;
    begin
      begin : BA int x [0:1] = '{44,44}; $display("A=%0d", x[0]); end
      begin : BB int x [0:1] = '{55,55}; $display("B=%0d", x[0]); end
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
    );
}

/// B0t4 — a PACKED STRUCT pair. Verilator-only: iverilog 13 rejects
/// `st_t x = '{a:44};` inside a `begin…end` (`syntax error` / `error: Syntax error in
/// variable list.`), so this value is pinned to verilator 5.052 alone, which prints
/// `A=44 B=55`. vita printed `A=55 B=55`.
#[test]
fn a_packed_struct_pair_verilator_only() {
    lines(
        r#"module top;
typedef struct packed { int a; } st_t;
  task t;
    begin
      begin : BA st_t x = '{a:44}; $display("A=%0d", x.a); end
      begin : BB st_t x = '{a:55}; $display("B=%0d", x.a); end
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
    );
}

/// B1t4 / B2t4 — the same struct shape in `task static` and `function void`, likewise
/// verilator-only.
#[test]
fn a_packed_struct_pair_in_a_static_task_verilator_only() {
    lines(
        r#"module top;
typedef struct packed { int a; } st_t;
  task static t;
    begin
      begin : BA st_t x = '{a:44}; $display("A=%0d", x.a); end
      begin : BB st_t x = '{a:55}; $display("B=%0d", x.a); end
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
    );
}

/// B2t4 — the same struct shape in a `function void`, likewise verilator-only.
#[test]
fn a_packed_struct_pair_in_a_void_function_verilator_only() {
    lines(
        r#"module top;
typedef struct packed { int a; } st_t;
  function void f;
    begin
      begin : BA st_t x = '{a:44}; $display("A=%0d", x.a); end
      begin : BB st_t x = '{a:55}; $display("B=%0d", x.a); end
    end
  endfunction
  initial begin f(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
    );
}

// ── (4) the invocation axis ─────────────────────────────────────────────────

/// C0r — four calls of one static task. The values repeat because a static
/// initializer runs once at t0, not per call.
#[test]
fn four_calls_of_one_static_task() {
    lines_without(
        r#"module top;
  task t;
    begin
      begin : BA int x = 44; $display("A=%0d", x); end
      begin : BB int x = 55; $display("B=%0d", x); end
    end
  endtask
  initial begin t(); t(); t(); t(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
        &["A=55"],
    );
}

/// C0p — the same task called from TWO processes.
#[test]
fn one_static_task_called_from_two_processes() {
    lines_without(
        r#"module top;
  task t;
    begin
      begin : BA int x = 44; $display("A=%0d", x); end
      begin : BB int x = 55; $display("B=%0d", x); end
    end
  endtask
  initial begin t(); end
  initial begin #0 t(); end
  initial begin #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
        &["A=55"],
    );
}

// ── (5) the risk cells ──────────────────────────────────────────────────────

/// E9 — the task lives in a module instantiated TWICE. Wrong per instance before, so
/// the instance prefix was never the missing key; correct per instance now.
#[test]
fn a_task_in_a_module_instantiated_twice() {
    lines_without(
        r#"module sub;
  task t;
    begin
      begin : BA int x = 44; $display("A=%0d", x); end
      begin : BB int x = 55; $display("B=%0d", x); end
    end
  endtask
  initial t();
endmodule
module top;
  sub u1(); sub u2();
  initial begin #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
        &["A=55"],
    );
}

/// E10 — a cross-subroutine read: `outer` has its own block `BC` and calls `inner`,
/// which holds the pair. `BC` was unaffected before and stays unaffected.
#[test]
fn a_block_in_a_calling_task_is_unaffected() {
    lines(
        r#"module top;
  task inner;
    begin
      begin : BA int x = 44; $display("A=%0d", x); end
      begin : BB int x = 55; $display("B=%0d", x); end
    end
  endtask
  task outer;
    begin
      begin : BC int x = 66; $display("C=%0d", x); end
      inner();
    end
  endtask
  initial begin outer(); #1 $finish; end
endmodule
"#,
        &["C=66", "A=44", "B=55"],
    );
}

/// E11 — each block wrapped in `if (1)`. Control-flow wrapping changes neither the
/// route nor the class.
#[test]
fn control_flow_wrapping_does_not_change_the_class() {
    lines(
        r#"module top;
  task t;
    begin
      if (1) begin : BA int x = 44; $display("A=%0d", x); end
      if (1) begin : BB int x = 55; $display("B=%0d", x); end
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
    );
}

/// E6 — a `#1` between the two blocks.
#[test]
fn a_delay_between_the_two_blocks() {
    lines(
        r#"module top;
  task t;
    begin
      begin : BA int x = 44; $display("A=%0d", x); end
      #1;
      begin : BB int x = 55; $display("B=%0d", x); end
    end
  endtask
  initial begin t(); #5 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
    );
}

/// E3 — `disable OUTER` from inside the first block. Both oracles print `A=44` and
/// NOTHING else: the disable of the enclosing labelled block stops the task body.
/// vita printed `A=55 B=55` — the pair aliased AND the disable did not stop the body.
/// Both halves are closed by the scoping (the `if (x == 44)` guard now sees 44).
#[test]
fn disable_of_an_enclosing_labelled_block() {
    lines_without(
        r#"module top;
  task t;
    begin : OUTER
      begin : BA int x = 44; $display("A=%0d", x); if (x == 44) disable OUTER; end
      begin : BB int x = 55; $display("B=%0d", x); end
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["A=44"],
        &["B=55", "A=55"],
    );
}

/// E5 — the pair inside a `fork … join`. The two oracles print the two blocks in
/// DIFFERENT order (iverilog `B=55` then `A=44`, verilator `A=44` then `B=55`), so
/// order is not oracle-decidable and only the two VALUES are pinned.
#[test]
fn the_pair_inside_a_fork_join_values_only() {
    lines_without(
        r#"module top;
  task t;
    begin
      fork
        begin : BA int x = 44; $display("A=%0d", x); end
        begin : BB int x = 55; $display("B=%0d", x); end
      join
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
        &["A=55"],
    );
}

/// E4 — a hierarchically called task routes `frame` and was correct before; it must
/// stay correct, because the `$blk$` wrap changes its frame slot count.
#[test]
fn a_hierarchically_called_task_stays_correct() {
    lines(
        r#"module sub;
  task t;
    begin
      begin : BA int x = 44; $display("A=%0d", x); end
      begin : BB int x = 55; $display("B=%0d", x); end
    end
  endtask
endmodule
module top;
  sub u();
  initial begin u.t(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
    );
}

/// E7 — a value-returning function routes `frame` and stays correct, return value
/// included.
#[test]
fn a_value_returning_function_stays_correct() {
    lines(
        r#"module top;
  function int f;
    begin
      begin : BA int x = 44; $display("A=%0d", x); end
      begin : BB int x = 55; $display("B=%0d", x); end
      f = 7;
    end
  endfunction
  int r;
  initial begin r = f(); $display("R=%0d", r); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55", "R=7"],
    );
}

/// R2 — the `auto_override` bitmask boundary. A `task automatic` with 62 body-top
/// locals plus two scoped static pairs and one `automatic` block-local declared LAST:
/// every scoped name appends a frame slot, so `locals_len` grows and the `automatic`
/// override bit for the last decl sits past slot 64 if the growth were mismanaged.
/// Measured identical PRE and POST and identical to both oracles, twice over (the
/// task is called twice).
#[test]
fn sixty_two_locals_plus_two_scoped_pairs_and_a_trailing_automatic() {
    let mut decls = String::new();
    let mut asg = String::new();
    for i in 0..62 {
        decls.push_str(&format!("      int L{i};\n"));
        asg.push_str(&format!("      L{i} = {i}; s = s + L{i};\n"));
    }
    let src = format!(
        r#"module top;
  task automatic t;
    int s;
    begin : TOP
{decls}      s = 0;
{asg}    end
    begin : BA int x = 44; $display("A=%0d", x); end
    begin : BB int x = 55; $display("B=%0d", x); end
    begin : BC automatic int w = 9; $display("W=%0d", w); end
    $display("S=%0d", s);
  endtask
  initial begin t(); t(); #1 $finish; end
endmodule
"#
    );
    lines(&src, &["A=44", "B=55", "W=9", "S=1891"]);
}

/// The EXT2-H `frame_array_local` marker fires only on the coalesce branch
/// (`frames_reserve.rs`), so scoping a name removes it from that branch. The shape it
/// protects — a block-local UNPACKED ARRAY shadowing a same-named outer scalar, with
/// an element write — is measured here on both routes and is correct, not silent: the
/// array gets its own net and the module scalar is untouched. Both oracles print
/// `A=7 8` / `M=3`.
#[test]
fn an_array_block_local_shadowing_a_module_scalar() {
    for kind in ["task t;", "task automatic t;"] {
        let src = format!(
            r#"module top;
  int y;
  {kind}
    begin : BA
      int y [0:1];
      y[0] = 7; y[1] = 8;
      $display("A=%0d %0d", y[0], y[1]);
    end
  endtask
  initial begin y = 3; t(); $display("M=%0d", y); #1 $finish; end
endmodule
"#
        );
        lines(&src, &["A=7 8", "M=3"]);
    }
}

// ── (6) loud → value: the shapes the E3009 scope-leak gate used to refuse ───

/// A03 — NESTED same-named blocks. The inner `x` shadows the outer one and the read
/// after the inner block is the OUTER `x`. Both oracles print `IN=55 OUT=44`; vita
/// refused it with `block-local `x` is referenced outside its `begin…end` block`.
#[test]
fn nested_same_named_blocks_in_a_subroutine() {
    lines(
        r#"module top;
  task t;
    begin
      begin : BA int x = 44;
        begin : BB int x = 55; $display("IN=%0d", x); end
        $display("OUT=%0d", x); end
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["IN=55", "OUT=44"],
    );
}

/// A05 — a block-local SHADOWING a module net. The read after the block is the MODULE
/// net. Both oracles print `A=44 M=11`.
#[test]
fn a_block_local_shadowing_a_module_net() {
    lines(
        r#"module top;
  int g = 11;
  task t;
    begin
      begin : BA int g = 44; $display("A=%0d", g); end
      $display("M=%0d", g);
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["A=44", "M=11"],
    );
}

/// A06 — a block-local shadowing a BODY-TOP local of the same subroutine. The read
/// after the block is the body-top one. Both oracles print `A=44 T=33`.
///
/// Note what does the work here: a subroutine FORMAL and a body-top local are NOT in
/// `module_names`, so the `shadows_module` admission rule does not fire for this name.
/// It is admitted by the static-initializer rule instead, which is why this cell moves
/// while a formal-shadowing one without an initializer would not.
#[test]
fn a_block_local_shadowing_a_body_top_local() {
    lines(
        r#"module top;
  task t;
    begin
      int tl = 33;
      begin : BA int tl = 44; $display("A=%0d", tl); end
      $display("T=%0d", tl);
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["A=44", "T=33"],
    );
}

/// A35 — the module-net shadow on the FRAME route.
#[test]
fn a_block_local_shadowing_a_module_net_on_the_frame_route() {
    lines(
        r#"module top;
  int g = 11;
  task automatic t;
    begin
      begin : BA int g = 44; $display("A=%0d", g); end
      $display("M=%0d", g);
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["A=44", "M=11"],
    );
}

/// A46 — the body-top-local shadow in a `function automatic void`.
#[test]
fn a_block_local_shadowing_a_body_top_local_in_an_automatic_function() {
    lines(
        r#"module top;
  function automatic void f;
    begin
      int tl = 33;
      begin : BA int tl = 44; $display("A=%0d", tl); end
      $display("T=%0d", tl);
    end
  endfunction
  initial begin f(); #1 $finish; end
endmodule
"#,
        &["A=44", "T=33"],
    );
}

// ── (7) what stays LOUD ─────────────────────────────────────────────────────

/// A `task automatic` whose block declares an unpacked ARRAY with an assignment
/// pattern is still refused — the pattern initializer is the pre-existing loud, and it
/// is loud on the different-name CONTROL too, so it is not this row's class. The
/// scope segment now appears in the `[in …]` suffix (`top.$func$t.$blk$48`), which is
/// what proves the decl is being elaborated under its own block scope.
#[test]
fn a_frame_unpacked_array_pattern_stays_loud() {
    loud(
        r#"module top;
  task automatic t;
    begin
      begin : BA int x [0:1] = '{44,44}; $display("A=%0d", x[0]); end
      begin : BB int x [0:1] = '{55,55}; $display("B=%0d", x[0]); end
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &[
            "an assignment pattern `'{…}` is supported only as the whole right-hand \
             side of an unpacked array assignment",
            "$blk$",
        ],
    );
}

/// An INITIALIZER-FREE sibling — `begin int x = 44; … end` beside `begin int x; … end`
/// with the second block READING `x` before assigning it.
///
/// ⚠️ This test was written as a RESIDUE pin: at §4.5.480 the no-initializer
/// declarator was admitted by no `AdmitReason`, so the name had ONE declaring span,
/// fell below the two-span bar in `compute_scoped_block_locals`, and both
/// declarations shared one net — vita printed `A=44 B=44` where both oracles print
/// `A=44 B=0`. The pin said "the day the admission rule is widened this test fails
/// and is updated". §2 Scoping row 2 widened it (the fifth `AdmitReason`,
/// `static_plain`, opt-in for subroutine bodies), so the expectation is updated
/// here to the value BOTH oracles give.
///
/// Re-measured on this exact design, POST: vita `A=44` / `B=0`; iverilog 13
/// `A=44` / `B=0`; verilator 5.052 `A=44` / `B=0`. The full battery lives in
/// `subroutine_plain_block_local.rs`.
#[test]
fn an_initializer_free_sibling_is_two_variables() {
    lines(
        r#"module top;
  task t;
    begin
      begin : BA int x = 44; $display("A=%0d", x); end
      begin : BB int x; $display("B=%0d", x); end
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=0"],
    );
}

/// Its different-name control is correct on both PRE and POST — `B=0`.
#[test]
fn the_initializer_free_control_is_correct() {
    lines(
        r#"module top;
  task t;
    begin
      begin : BA int x = 44; $display("A=%0d", x); end
      begin : BB int y; $display("B=%0d", y); end
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=0"],
    );
}

/// A framed STATIC task loses static retention across calls — a SEPARATE root, filed
/// as its own §2 row. Its different-name control is equally wrong, which is what
/// proves it is not the coalesce: both print `A=45 B=56` twice where both oracles
/// print `A=45 B=56` then `A=46 B=57`.
#[test]
fn a_framed_static_task_loses_retention_separate_root() {
    lines(
        r#"module sub;
  task t;
    begin
      begin : BA int x = 44; x = x + 1; $display("A=%0d", x); end
      begin : BB int y = 55; y = y + 1; $display("B=%0d", y); end
    end
  endtask
endmodule
module top;
  sub u();
  initial begin u.t(); u.t(); #1 $finish; end
endmodule
"#,
        &["A=45", "B=56"],
    );
}

// ── (8) the out-of-scope collectors, measured UNCHANGED ─────────────────────

/// A PACKAGE `task automatic` with two initialized sibling block-locals. Both oracles
/// print `A=44 B=55` and vita has matched them throughout: the frame body emits a
/// block's decl-inits at BLOCK ENTRY, which masked the shared slot even before the
/// package bodies were classified. (Updated prose only — §2 Scoping row 3 since fed
/// the package bodies to the same classifier; the value is unchanged.)
#[test]
fn a_package_task_is_unchanged() {
    lines(
        r#"package pk;
  task automatic pt;
    begin : BA int x = 44; $display("A=%0d", x); end
    begin : BB int x = 55; $display("B=%0d", x); end
  endtask
endpackage
module top;
  import pk::*;
  initial begin pt(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
    );
}

/// A STATIC package task was a RESIDUE of the slice that wrote this file: its sibling
/// block-locals coalesced onto one net and vita printed `A=55 B=55` where iverilog 13
/// and verilator 5.052 both print `A=44 B=55`. §2 Scoping row 3 closed it by feeding a
/// package routine's body — injected into the caller module's `func_table`/`task_table`
/// — to the same `compute_scoped_block_locals` walk the module's own subroutines get
/// (`instance.rs` step 3.6a). Re-measured on both oracles: `A=44 B=55`. The test name
/// is kept from the residue era; the full cell set for the row lives in
/// `package_subroutine_block_local.rs`.
#[test]
fn a_static_package_task_is_a_recorded_residue() {
    lines(
        r#"package pk;
  task pt;
    begin : BA int x = 44; $display("A=%0d", x); end
    begin : BB int x = 55; $display("B=%0d", x); end
  endtask
endpackage
module top;
  import pk::*;
  initial begin pt(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
    );
}

/// An INTERFACE subroutine is refused outright (`functions/tasks inside an interface
/// are outside the MVP`) on both PRE and POST — the shape never reaches either
/// reserver, so this slice cannot move it. Both oracles run it and print `A=44 B=55`.
#[test]
fn an_interface_task_is_unchanged_and_loud() {
    loud(
        r#"interface ifc;
  task it;
    begin : BA int x = 44; $display("A=%0d", x); end
    begin : BB int x = 55; $display("B=%0d", x); end
  endtask
endinterface
module top;
  ifc i();
  initial begin i.it(); #1 $finish; end
endmodule
"#,
        &["functions/tasks inside an interface are outside the MVP"],
    );
}

/// A CLASS method's sibling block-locals go through `classes.rs`'s own collector.
/// Measured identical PRE and POST: still LOUD (`undeclared net/variable
/// `$class$C$m.x``), while both oracles print `A=44 B=55`. Out of scope, filed as its
/// own §2 row.
#[test]
fn a_class_method_is_unchanged_and_loud() {
    loud(
        r#"module top;
  class C;
    function void m();
      begin : BA int x = 44; $display("A=%0d", x); end
      begin : BB int x = 55; $display("B=%0d", x); end
    endfunction
  endclass
  C c;
  initial begin c = new(); c.m(); #1 $finish; end
endmodule
"#,
        &["undeclared net/variable `$class$C$m.x`"],
    );
}

/// `compute_coalesced_block_locals` is deliberately NOT given the subroutine walk.
/// This is the cell that decides it: a task declaring `v` in two blocks beside an
/// `initial` block that declares its own `v` and reads it unassigned. Both oracles
/// print `p=0 t1=1 t2=2`. Feeding the subroutine bodies into that classifier turns the
/// module process's own `v` — a lone block-local with a net of its own — into
/// `error[VITA-E3009] … shares one flattened net with a same-named block-local in
/// another block but is READ before it is assigned here`, a correct → loud regression.
#[test]
fn module_process_block_local_beside_a_task_of_the_same_name() {
    lines(
        r#"module top;
  task t;
    begin : BA int v; v = 1; $display("t1=%0d", v); end
    begin : BB int v; v = 2; $display("t2=%0d", v); end
  endtask
  initial begin
    begin : P1
      int v;
      $display("p=%0d", v);
    end
    t();
    #1 $finish;
  end
endmodule
"#,
        &["p=0", "t1=1", "t2=2"],
    );
}

/// A subroutine with NO same-named sibling block-locals is untouched: a lone
/// block-local is not admitted by the two-span bar, so `block_local_scope_prefix`
/// returns `None` and the reserve takes the pre-existing bare-name path.
#[test]
fn a_lone_block_local_is_not_scoped() {
    lines(
        r#"module top;
  task t;
    begin
      begin : BA int x = 44; $display("A=%0d", x); end
      begin : BB int y = 55; $display("B=%0d", y); end
    end
  endtask
  initial begin t(); #1 $finish; end
endmodule
"#,
        &["A=44", "B=55"],
    );
}
