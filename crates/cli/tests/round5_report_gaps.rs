//! Round-5 external-report gaps (single-file repros → two fixes).
//!
//! From an external tester's round-5 report against 2309877 (the round-4 fixes
//! verified: G/D/E1 now PASS). Round-5 closes the two remaining actionable items
//! correct-or-loud:
//!
//! - **DUP** (elaborate): the SAME-named `automatic` block-local declared in TWO
//!   DISJOINT procedural blocks (two `always`/`initial` blocks each declaring
//!   `automatic int idx;`). v1 flattens block-locals to the module namespace by
//!   BARE name, so the pair would alias → the second was rejected E3009. Round-5
//!   gives each such colliding automatic its own `$blk$<span>` transparent scope
//!   segment (like `$func$`/`$itask$`) so the two become DISTINCT nets. Tightly
//!   guarded by a pure-AST pre-scan (`compute_scoped_block_locals`): only mutually
//!   disjoint blocks, no module-net collision, no nested scoped blocks — every
//!   uncovered edge falls through to the pre-existing loud E3009. Distinct storage
//!   is observable with timing (a delayed read must see its OWN block's write, not
//!   the other's). No iverilog oracle for the automatic lifetime (13.0 "sorry");
//!   verified by a plain-static twin + the timing differential.
//! - **B** (parser + elaborate): a body-local `typedef enum` in a FUNCTION body is
//!   now supported. The parser already resolves the type NAME and `e'(x)` casts
//!   from its scratch maps; round-5 threads the enum's LABELS (via a new AST
//!   `FunctionDef/TaskDef.body_enums` slot) to elaborate, which registers them as
//!   integer constants scoped to the function (frame: under `$func$<name>`; inline:
//!   under the caller prefix bounded by the reduction) — so they resolve inside the
//!   body but do NOT leak to the module. A body-local enum in a bare `begin/end`
//!   block (no carrier) stays honest-loud. iverilog 13.0 segfaults on body-local
//!   enums (no oracle) — verified by hand + self-consistency (label fold values).
//!
//! Parser + elaborate; AST `.vu`-hash re-pins once (new `body_enums` slot),
//! sim-ir/format_version 19 UNCHANGED, IR-0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src` through one-shot vita; return (stdout, stderr, success).
fn run(src: &str) -> (String, String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r5g_{}_{n}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let f = d.join("t.sv");
    std::fs::write(&f, src).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_vita"))
        .arg(f.to_str().unwrap())
        .current_dir(&d)
        .output()
        .expect("run vita");
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.success(),
    )
}

fn err_count(e: &str) -> usize {
    e.matches("error[").count()
}

/// stdout with the trailing engine status lines removed (`simulation ended …` /
/// `$finish called …`), leaving just the `$display` output.
fn disp(o: &str) -> String {
    let mut s = String::new();
    for l in o.lines() {
        if l.starts_with("simulation ended") || l.contains("$finish called") {
            continue;
        }
        s.push_str(l);
        s.push('\n');
    }
    s
}

// ─────────────────────────────── Gap B ───────────────────────────────

#[test]
fn gap_b_function_enum_frame_supported() {
    // An `automatic` function (FRAME path) with a body-local enum; labels used in a
    // comparison. {LO=1, HI=2}: decode(2)==HI ⇒ f, decode(1)==LO ⇒ a, else 0.
    let (o, _e, ok) = run("module tb;\n\
         logic [1:0] s; logic [3:0] y;\n\
         function automatic logic [3:0] decode(input logic [1:0] x);\n\
           typedef enum logic [1:0] { LO = 2'd1, HI = 2'd2 } e;\n\
           e v; v = e'(x);\n\
           if (v == HI) return 4'hF; else if (v == LO) return 4'hA; else return 4'h0;\n\
         endfunction\n\
         assign y = decode(s);\n\
         initial begin\n\
           s = 2'd0; #1 $display(\"%h\", y);\n\
           s = 2'd1; #1 $display(\"%h\", y);\n\
           s = 2'd2; #1 $display(\"%h\", y);\n\
         end endmodule");
    assert!(ok, "frame-path body-local enum must elaborate + simulate");
    assert_eq!(
        disp(&o),
        "0\na\nf\n",
        "label folds LO=1→a, HI=2→f; got:\n{o}"
    );
}

#[test]
fn gap_b_function_enum_inline_supported() {
    // A STATIC function (INLINE path) with a body-local enum; labels used in the
    // straight-line return expression. Same LO=1/HI=2 fold.
    let (o, _e, ok) = run("module tb;\n\
         logic [1:0] s; logic [3:0] y;\n\
         function logic [3:0] decode(input logic [1:0] x);\n\
           typedef enum logic [1:0] { LO = 2'd1, HI = 2'd2 } e;\n\
           return (x == HI) ? 4'hF : (x == LO) ? 4'hA : 4'h0;\n\
         endfunction\n\
         assign y = decode(s);\n\
         initial begin\n\
           s = 2'd2; #1 $display(\"%h\", y);\n\
           s = 2'd1; #1 $display(\"%h\", y);\n\
         end endmodule");
    assert!(ok, "inline-path body-local enum must elaborate + simulate");
    assert_eq!(disp(&o), "f\na\n", "HI=2→f, LO=1→a; got:\n{o}");
}

#[test]
fn gap_b_enum_labels_do_not_leak() {
    // The labels must be visible ONLY inside the function — referencing one at
    // module scope is undeclared (no leak). Correct-or-loud (E3010).
    let (_o, e, ok) = run("module m (output logic [3:0] y);\n\
         function automatic logic [3:0] f(input logic [1:0] x);\n\
           typedef enum logic [1:0] { LO = 1, HI = 2 } e;\n\
           return (x == HI) ? 4'hF : 4'h0;\n\
         endfunction\n\
         assign y = HI;\n\
         endmodule");
    assert!(!ok, "an enum label must not leak out of its function");
    assert!(
        e.contains("HI"),
        "expected an undeclared-`HI` diagnostic:\n{e}"
    );
}

#[test]
fn gap_b_same_label_two_functions_are_distinct() {
    // Two functions each defining a label `K` with a DIFFERENT value must each use
    // their OWN — no module-scope leak/collision (last-wins would be silent-wrong).
    // fa: K=1 → fa(1)=a ; fb: K=3 → fb(1)=0 (1 != 3), fb(3)=b.
    let (o, _e, ok) = run("module tb;\n\
         logic [3:0] a1, b1, b3;\n\
         function automatic logic [3:0] fa(input logic [1:0] x);\n\
           typedef enum logic [1:0] { K = 2'd1 } e; return (x == K) ? 4'hA : 4'h0;\n\
         endfunction\n\
         function automatic logic [3:0] fb(input logic [1:0] x);\n\
           typedef enum logic [1:0] { K = 2'd3 } e; return (x == K) ? 4'hB : 4'h0;\n\
         endfunction\n\
         assign a1 = fa(2'd1); assign b1 = fb(2'd1); assign b3 = fb(2'd3);\n\
         initial begin #1 $display(\"%h %h %h\", a1, b1, b3); end endmodule");
    assert!(ok, "distinct per-function enum labels must elaborate");
    assert_eq!(disp(&o), "a 0 b\n", "fa.K=1, fb.K=3 each local; got:\n{o}");
}

#[test]
fn gap_b_block_enum_stays_loud() {
    // A body-local enum in a bare `begin/end` block (no `body_enums` carrier) stays
    // honest-loud — correct-or-loud.
    let (_o, e, ok) = run("module m (input logic [1:0] s, output logic [1:0] y);\n\
         always_comb begin\n\
           typedef enum logic [1:0] { A = 0, B = 1 } e;\n\
           e v; v = e'(s); y = v;\n\
         end endmodule");
    assert!(!ok, "a begin/end block-local enum must stay loud");
    assert!(
        e.contains("body-local enum typedef is unsupported"),
        "expected the enum-cut message:\n{e}"
    );
}

#[test]
fn gap_b_fail_repro_now_elaborates() {
    // The exact round-5 B_FAIL repro (bare-name call isolates the enum) now
    // elaborates + simulates (was E2002 parse-loud).
    let (_o, _e, ok) = run("package bp;\n\
           function automatic logic [1:0] f(input logic [1:0] x);\n\
             typedef enum logic [1:0] { A = 2'd0, B = 2'd1 } e;\n\
             e v; v = e'(x); return logic'(v);\n\
           endfunction\n\
         endpackage\n\
         module m import bp::*; (input logic [1:0] s, output logic [1:0] y);\n\
           assign y = f(s); endmodule");
    assert!(ok, "B_FAIL repro must now elaborate + simulate");
}

// ─────────────────────────────── DUP ───────────────────────────────

#[test]
fn dup_two_blocks_same_auto_local_now_supported() {
    // The exact round-5 DUP_FAIL repro: two `always_comb` blocks each declaring
    // `automatic int idx;`. Was E3009; now each gets its own `$blk$` scope.
    let (_o, _e, ok) = run(
        "module m (input logic [3:0] s, output logic [7:0] y0, output logic [7:0] y1);\n\
         always_comb begin automatic int idx; idx = s + 1; y0 = idx[7:0]; end\n\
         always_comb begin automatic int idx; idx = s + 2; y1 = idx[7:0]; end\n\
         endmodule",
    );
    assert!(
        ok,
        "two disjoint blocks reusing an automatic local name must elaborate"
    );
}

#[test]
fn dup_distinct_storage_proven_by_timing() {
    // Distinct storage is OBSERVABLE with timing: each block's delayed read must see
    // its OWN write. Aliasing onto one net would give y0=200 (B's t0 write wins);
    // distinct nets give y0=100, y1=200.
    let (o, _e, ok) = run("module tb;\n\
         int y0, y1;\n\
         initial begin automatic int idx; idx = 100; #2 y0 = idx; end\n\
         initial begin automatic int idx; idx = 200; #2 y1 = idx; end\n\
         initial #3 $display(\"%0d %0d\", y0, y1);\n\
         endmodule");
    assert!(ok, "timing DUP case must elaborate");
    assert_eq!(
        disp(&o),
        "100 200\n",
        "distinct storage: aliasing would give 200 200; got:\n{o}"
    );
}

#[test]
fn dup_ok_renamed_still_works() {
    // The report's DUP_OK workaround (rename the second local) is unaffected.
    let (_o, _e, ok) = run(
        "module m (input logic [3:0] s, output logic [7:0] y0, output logic [7:0] y1);\n\
         always_comb begin automatic int idx;   idx   = s + 1; y0 = idx[7:0];   end\n\
         always_comb begin automatic int idx_b; idx_b = s + 2; y1 = idx_b[7:0]; end\n\
         endmodule",
    );
    assert!(ok, "the renamed workaround must still elaborate");
}

#[test]
fn dup_collision_with_real_net_stays_loud() {
    // A block-local automatic whose name collides with a REAL module net (not
    // another block-local) must STAY loud — aliasing a real net is the round-4 GAP-D
    // hazard the fix must not weaken.
    let (_o, e, ok) = run(
        "module m (input logic [3:0] s, output logic [7:0] y0, output logic [7:0] y1);\n\
         int idx;\n\
         always_comb begin automatic int idx; idx = s + 1; y0 = idx[7:0]; end\n\
         always_comb begin automatic int idx; idx = s + 2; y1 = idx[7:0]; end\n\
         endmodule",
    );
    assert!(!ok, "collision with a real module net must stay loud");
    assert!(err_count(&e) >= 1, "expected an E3009 collision:\n{e}");
}

#[test]
fn dup_nested_shadow_stays_loud() {
    // A nested block re-declaring the same automatic name (SHADOWING, not disjoint)
    // is excluded from scoping and stays loud — v1 has no nested per-block shadow.
    let (_o, _e, ok) = run("module tb;\n\
         int y;\n\
         initial begin\n\
           automatic int idx; idx = 1;\n\
           begin automatic int idx; idx = 2; y = idx; end\n\
         end\n\
         initial #1 $display(\"%0d\", y);\n\
         endmodule");
    assert!(
        !ok,
        "a nested same-name automatic (shadowing) must stay loud"
    );
}

#[test]
fn dup_read_before_write_still_loud() {
    // A scoped block-local still passes per-entry DEFINITE-ASSIGNMENT: a read before
    // its first write (observing the per-entry reset) stays loud even when scoped.
    let (_o, _e, ok) = run("module tb;\n\
         int y0, y1;\n\
         initial begin automatic int idx; idx = 5; y0 = idx; end\n\
         initial begin automatic int idx; y1 = idx; idx = 9; end\n\
         initial #1 $display(\"%0d %0d\", y0, y1);\n\
         endmodule");
    assert!(
        !ok,
        "read-before-write of a scoped automatic must stay loud"
    );
}

#[test]
fn dup_static_locals_still_coalesce() {
    // Two DISJOINT blocks reusing a STATIC (non-automatic) local name still coalesce
    // onto one net (pre-existing, correct — they never overlap in time). Unchanged.
    let (o, _e, ok) = run("module tb;\n\
         int y0, y1;\n\
         initial begin int t; t = 3; y0 = t; end\n\
         initial begin int t; t = 4; y1 = t; end\n\
         initial #1 $display(\"%0d %0d\", y0, y1);\n\
         endmodule");
    assert!(ok, "static same-name block-locals still coalesce");
    assert_eq!(
        disp(&o),
        "3 4\n",
        "each block writes before read; got:\n{o}"
    );
}

// ───────── Adversarial-review regressions (R1 fixes) ─────────

#[test]
fn gap_b_task_enum_frame_shadows_module() {
    // R1 (B-soundness): a `task automatic` (FRAME path) body-local enum label must
    // SHADOW a same-named module localparam inside the task — NOT leak the module
    // value in (that was a silent-wrong: task labels were unregistered). Local
    // IDLE=0/RUN=1 (module IDLE=100/RUN=200) ⇒ o = IDLE*1000+RUN = 1.
    let (o, _e, ok) = run("module m;\n\
         localparam int IDLE = 100;\n\
         localparam int RUN = 200;\n\
         task automatic t(output int o);\n\
           typedef enum { IDLE, RUN } e_t; o = IDLE*1000 + RUN;\n\
         endtask\n\
         integer r; initial begin t(r); $display(\"%0d\", r); end endmodule");
    assert!(ok, "task-frame body enum must elaborate");
    assert_eq!(
        disp(&o),
        "1\n",
        "task-local IDLE=0,RUN=1 shadow module; got:\n{o}"
    );
}

#[test]
fn gap_b_task_enum_inline_shadows_module() {
    // R1 (B-soundness): a static `task` (INLINE path) body-local enum label must
    // shadow a same-named module localparam. Local P=5, module P=100 ⇒ o = 5.
    let (o, _e, ok) = run("module m;\n\
         localparam int P = 100;\n\
         task t(output int o);\n\
           typedef enum { P = 5 } e_t; o = P;\n\
         endtask\n\
         integer r; initial begin t(r); $display(\"%0d\", r); end endmodule");
    assert!(ok, "task-inline body enum must elaborate");
    assert_eq!(
        disp(&o),
        "5\n",
        "task-local P=5 shadows module P=100; got:\n{o}"
    );
}

#[test]
fn dup_multiname_decl_scoped_together() {
    // R1 (DUP-soundness): a MULTI-name `automatic` decl where only ONE name collides
    // — the WHOLE decl is scoped (any-name rule), so the colliding name never keeps a
    // bare module net that a later same-named STATIC local could alias. Block `a`'s
    // `idx` (scoped, because it also appears in block `b`) must read 100, independent
    // of block `x`'s static `idx = 500` (was silent-wrong: `a` read 500).
    let (o, _e, ok) = run("module top;\n\
         int ya, yx;\n\
         initial begin : a automatic int idx, jdx; idx = 100; jdx = 0; #5 ya = idx; end\n\
         initial begin : x int idx; #2 idx = 500; #10 yx = idx; end\n\
         initial begin : b automatic int idx; idx = 200; end\n\
         initial #20 $display(\"%0d %0d\", ya, yx);\n\
         endmodule");
    assert!(ok, "multi-name decl with a colliding name must elaborate");
    assert_eq!(
        disp(&o),
        "100 500\n",
        "scoped block-a idx=100 distinct from static idx=500; got:\n{o}"
    );
}

#[test]
fn gap_b_label_vs_inner_block_local_is_loud() {
    // R2 (B-soundness): a body enum label whose name equals an inner `begin/end`
    // block-local (which should shadow it per IEEE §6.21) is loud-rejected — vita
    // resolves the enclosing label OVER the inner net, so reject rather than
    // silently mis-shadow (was silent r=9; the block-local intends 3).
    let (_o, e, ok) = run("module m;\n\
         task automatic t(output int o); typedef enum {v=9} e_t;\n\
           begin int v; v=3; o=v; end\n\
         endtask\n\
         integer r; initial begin t(r); $display(\"%0d\",r); end endmodule");
    assert!(
        !ok,
        "label colliding with an inner block-local must be loud"
    );
    assert!(
        e.contains("shares its name"),
        "expected the label/block-local collision message:\n{e}"
    );
}

#[test]
fn gap_b_label_and_block_local_no_collision_ok() {
    // The guard is PRECISE: a label `v` plus a DIFFERENTLY-named block-local `w`
    // resolves normally (label v=9 used, block-local w=3) ⇒ 12.
    let (o, _e, ok) = run("module m;\n\
         task automatic t(output int o); typedef enum {v=9} e_t;\n\
           begin int w; w=3; o=w+v; end\n\
         endtask\n\
         integer r; initial begin t(r); $display(\"%0d\",r); end endmodule");
    assert!(ok, "non-colliding label + block-local must work");
    assert_eq!(disp(&o), "12\n", "w=3 + v=9 = 12; got:\n{o}");
}
