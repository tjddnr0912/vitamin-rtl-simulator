//! Round-4 external-report gaps (single-file repros → three fixes).
//!
//! From an external tester's round-4 report against 692998a. The round-3 G/D
//! fixes narrowed each gap to one precise residual trigger; round-4 closes them
//! correct-or-loud:
//!
//! - **G** (elaborate): a generate-scope `localparam R = ROT[g]` reading a
//!   PACKAGE-scope unpacked-array parameter (`package p; localparam int ROT[…]`),
//!   named either by its bare name via `import p::*` or explicitly as `p::ROT`.
//!   Round-3 folded module-local arrays only; the package array lowers to a
//!   `$pkg$p` net, so its elements were not in the module-scope table. Captured
//!   in `pkg_array_const_vals` and resolved by `const_array_vals_of_base`.
//!   Oracle: byte-identical to the round-3-validated module-local array read.
//! - **D** (elaborate): a block-local `automatic` written & read under the SAME
//!   conditional guard inside a loop (`if (c) begin slot = …; … slot … end`).
//!   The round-3 accept required a top-level dominating write; round-4 replaces
//!   it with a guard-aware DEFINITE-ASSIGNMENT analysis (`da_stmt`) that accepts
//!   any local definitely written before every read on every path (its per-entry
//!   reset is then unobservable, so the v1 static flattening is byte-identical).
//!   Still conservative: read-before-write on ANY path stays loud. Oracle: an
//!   automatic-FUNCTION twin (iverilog-legal) + a plain-static twin (which vita
//!   flattens identically) — both agree with vita's accepted automatic.
//! - **B / `pkg::f(args)`** (parser): the report's B repro also exercises a
//!   package-scoped SUBROUTINE CALL `bp::f(s)`, a separate unsupported feature
//!   that used to desync into a spurious "expected module item" cascade. It is
//!   now a CLEAN single loud (the balanced `(args)` is consumed) that points at
//!   the working `import`-and-call-by-name path. The body-local enum typedef
//!   itself stays a v1 cut (LOUD — iverilog 13.0 segfaults, no oracle).
//!
//! Pure parser (`pkg::f`) + elaborate (G/D); no AST field added, `.vu`/format
//! unchanged, IR-0.
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

/// Run `src` through one-shot vita; return (stdout, stderr, success).
fn run(src: &str) -> (String, String, bool) {
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir().join(format!("vita_r4g_{}_{n}", std::process::id()));
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

// ─────────────────────────────── GAP G ───────────────────────────────
// A per-lane rotate whose amount is a package array element. ROT={0,1,3,5};
// total rotate 0+1+3+5 = 9 ≡ 1 (mod 8) ⇒ o = rotl(i,1); rotl(8'hA5,1)=8'h4B.

const G_BODY: &str = "\n\
    logic [7:0] st [0:4]; assign st[0] = i; genvar g;\n\
    generate for (g = 0; g < 4; g++) begin : gl\n\
        localparam int R = SEL;\n\
        if (R == 0) begin : r0 assign st[g+1] = st[g];\n\
        end else begin : rn assign st[g+1] = (st[g] << R) | (st[g] >> (8 - R)); end\n\
    end endgenerate\n\
    assign o = st[4];\n";

fn g_top() -> &'static str {
    "module top; logic [7:0] i, o; dut d(.i(i), .o(o));\n\
     initial begin i=8'hA5; #1; $display(\"o=%02h\", o); end endmodule\n"
}

#[test]
fn gap_g_package_array_via_wildcard_import() {
    // The report's G_FAIL (bare `ROT[g]` via `import gp::*`).
    let src = format!(
        "package gp; localparam int ROT [0:3] = '{{0, 1, 3, 5}}; endpackage\n\
         module dut import gp::*; (input logic [7:0] i, output logic [7:0] o);{}endmodule\n{}",
        G_BODY.replace("SEL", "ROT[g]"),
        g_top()
    );
    let (o, e, ok) = run(&src);
    assert!(ok, "package-array generate fold must elaborate: {e}");
    assert!(o.contains("o=4b"), "expected o=4b, got: {o}");
}

#[test]
fn gap_g_package_array_explicit_qualified() {
    // Explicit `gp::ROT[g]` (no import) folds identically.
    let src = format!(
        "package gp; localparam int ROT [0:3] = '{{0, 1, 3, 5}}; endpackage\n\
         module dut (input logic [7:0] i, output logic [7:0] o);{}endmodule\n{}",
        G_BODY.replace("SEL", "gp::ROT[g]"),
        g_top()
    );
    let (o, e, ok) = run(&src);
    assert!(ok, "explicit `gp::ROT[g]` must elaborate: {e}");
    assert!(o.contains("o=4b"), "expected o=4b, got: {o}");
}

#[test]
fn gap_g_module_local_array_still_folds() {
    // Round-3 module-local + derived-index path stays green (regression guard).
    let src = format!(
        "module dut (input logic [7:0] i, output logic [7:0] o);\n\
         localparam int ROT [0:3] = '{{0, 1, 3, 5}};{}endmodule\n{}",
        G_BODY.replace("SEL", "ROT[g]"),
        g_top()
    );
    let (o, _e, ok) = run(&src);
    assert!(ok && o.contains("o=4b"), "module-local fold regressed: {o}");
}

#[test]
fn gap_g_package_array_matches_module_local() {
    // Differential: the package read and the module-local read of the SAME array
    // and datapath produce byte-identical output (the local path is round-3
    // iverilog-validated). Both must print o=4b.
    let mk = |decl: &str, sel: &str, imp: &str| {
        format!(
            "package gp; localparam int ROT [0:3] = '{{0, 1, 3, 5}}; endpackage\n\
             module dut {imp}(input logic [7:0] i, output logic [7:0] o);{}{}endmodule\n{}",
            decl,
            G_BODY.replace("SEL", sel),
            g_top()
        )
    };
    let (op, _, okp) = run(&mk("", "ROT[g]", "import gp::*; "));
    let (ol, _, okl) = run(&mk(
        "localparam int ROT [0:3] = '{0, 1, 3, 5};",
        "ROT[g]",
        "",
    ));
    assert!(okp && okl, "both variants must elaborate");
    let vp = op.lines().find(|l| l.contains("o=")).unwrap_or("");
    let vl = ol.lines().find(|l| l.contains("o=")).unwrap_or("");
    assert_eq!(vp, vl, "package read must equal module-local read");
    assert!(vp.contains("o=4b"), "expected o=4b, got {vp}");
}

#[test]
fn gap_g_package_array_oob_index_is_loud() {
    // Out-of-range element read is not captured → folds None → loud (not a
    // silent 0).
    let (_o, e, ok) = run(
        "package gp; localparam int ROT [0:3] = '{0, 1, 3, 5}; endpackage\n\
         module dut import gp::*; (output logic [7:0] o);\n\
         localparam int R = ROT[9]; assign o = R[7:0]; endmodule\n",
    );
    assert!(!ok, "OOB package-array element must be loud");
    assert!(e.contains("error["), "expected a loud error: {e}");
}

#[test]
fn gap_g_package_array_descending_is_loud() {
    // A descending package array `[3:0]` is not captured (round-4 keeps the
    // round-3 shape rule) → element read is loud, never a silent value.
    let (_o, e, ok) = run(
        "package gp; localparam int ROT [3:0] = '{0, 1, 3, 5}; endpackage\n\
         module dut import gp::*; (output logic [7:0] o);\n\
         localparam int R = ROT[1]; assign o = R[7:0]; endmodule\n",
    );
    assert!(!ok, "descending package array element must be loud");
    assert!(e.contains("error["), "expected a loud error: {e}");
}

#[test]
fn gap_g_local_array_shadowing_import_is_loud_not_leaked() {
    // Adversarial find: a module-local array of an UNCAPTURED shape (descending)
    // that shadows a same-named wildcard-imported package array must NOT silently
    // fold the IMPORTED array. Per SV shadowing the local wins; it is uncaptured,
    // so the const read is LOUD — never the imported value (20). Runtime reads the
    // local (92). Guard: a const fold that returned 20 here would silently
    // contradict the runtime read.
    let (_o, e, ok) = run(
        "package gp; localparam int ROT [0:3] = '{10, 20, 30, 40}; endpackage\n\
         module m import gp::*; (output logic [31:0] o);\n\
         localparam int ROT [3:0] = '{90, 91, 92, 93};\n\
         localparam int R = ROT[1];\n\
         assign o = R; endmodule\n",
    );
    assert!(
        !ok,
        "local array shadowing an imported array must be loud, not leaked"
    );
    assert!(
        e.contains("error["),
        "expected a loud error, not a silent imported fold: {e}"
    );
}

#[test]
fn gap_g_local_scalar_shadowing_import_reads_the_local_bit_not_the_imported_element() {
    // A local SCALAR param of the same name as a wildcard-imported array shadows it
    // (local-wins), so `ROT[1]` is a BIT-select of 99 = 1, never the imported
    // element 20. That property is what this test has always been about.
    //
    // ⚠️ IT USED TO ASSERT **LOUD**, and that was a capability gap pinned as intent:
    // the const domain had no scalar bit-select arm, so the only way to keep the
    // fold from disagreeing with the runtime read was to refuse both. The
    // param-select fold closed that gap, so the two now AGREE on 1 — which is the
    // stronger form of the same guarantee, and the one both oracles give
    // (`localparam int ROT = 99; ROT[1]` is 1 in iverilog 13.0 and verilator 5.050;
    // for the shape below iverilog rejects unpacked array parameters outright, so
    // verilator is the oracle and it answers 1).
    let (o, e, ok) = run(
        "package gp; localparam int ROT [0:3] = '{10, 20, 30, 40}; endpackage\n\
         module dut import gp::*; (output logic [7:0] o);\n\
         localparam int ROT = 99;\n\
         localparam int R = ROT[1];\n\
         assign o = R[7:0];\n\
         initial begin #1; $display(\"R=%0d\", o); end endmodule\n",
    );
    assert!(ok, "expected success, got: {e}");
    assert!(
        o.contains("R=1"),
        "must read bit 1 of the LOCAL 99, not the imported element 20: {o}{e}"
    );
}

// ─────────────────────────────── GAP D ───────────────────────────────
// A block-local `automatic slot` written then read under a shared `if (k<s)`
// guard in a loop. y bit `slot`(=k+1) is set for each k<s ⇒ y = {1..s} bits.

const D_DUT: &str = "module dut (input logic [3:0] s, output logic [7:0] y);\n\
    always_comb begin\n\
        automatic int slot;\n\
        y = '0;\n\
        for (int k = 0; k < 4; k++) begin\n\
            if (k < s) begin\n\
                slot = k + 1;\n\
                if (slot < 8) y[slot] = 1'b1;\n\
            end\n\
        end\n\
    end\nendmodule\n";

#[test]
fn gap_d_guarded_write_before_read_in_loop_is_correct() {
    // The report's D_FAIL: accepted AND correct. s=0..4 ⇒ y=00,02,06,0e,1e
    // (bits 1..s set); matches the automatic-function / plain-static twins.
    let src = format!(
        "{D_DUT}module top; logic [3:0] s; logic [7:0] y; dut d(.s(s),.y(y));\n\
         initial begin for (int t=0;t<5;t++) begin s=t[3:0]; #1; $display(\"y=%02h\",y); end end\n\
         endmodule\n"
    );
    let (o, e, ok) = run(&src);
    assert!(
        ok,
        "guarded write-before-read automatic must elaborate: {e}"
    );
    let ys: Vec<&str> = o.lines().filter(|l| l.starts_with("y=")).collect();
    assert_eq!(
        ys,
        vec!["y=00", "y=02", "y=06", "y=0e", "y=1e"],
        "guard-dominated automatic must match static/iverilog twins"
    );
}

#[test]
fn gap_d_static_flatten_equals_automatic() {
    // The whole point of the accept: the v1 static flattening (a plain `int
    // slot`) is byte-identical to the `automatic` for this shape. Run both.
    let top = "module top; logic [3:0] s; logic [7:0] y; dut d(.s(s),.y(y));\n\
        initial begin for (int t=0;t<5;t++) begin s=t[3:0]; #1; $display(\"y=%02h\",y); end end\n\
        endmodule\n";
    let (oa, _, oka) = run(&format!("{D_DUT}{top}"));
    let (os, _, oks) = run(&format!(
        "{}{top}",
        D_DUT.replace("automatic int slot;", "int slot;")
    ));
    assert!(oka && oks, "both automatic and static must elaborate");
    let ya: Vec<&str> = oa.lines().filter(|l| l.starts_with("y=")).collect();
    let ysx: Vec<&str> = os.lines().filter(|l| l.starts_with("y=")).collect();
    assert_eq!(ya, ysx, "automatic accept must equal the static flattening");
}

#[test]
fn gap_d_both_branch_assign_then_read_is_accepted() {
    // `if (c) slot=…; else slot=…;` assigns on EVERY path ⇒ definitely assigned
    // ⇒ accepted (the if/else merge in `da_stmt`).
    let (_o, e, ok) = run("module m (input logic [7:0] s, output logic [7:0] y);\n\
         always_comb begin automatic int slot; y='0;\n\
         if (s > 0) slot = 5; else slot = 2; y[0] = (slot > 3); end endmodule\n");
    assert!(ok, "both-branch assignment must be accepted: {e}");
}

#[test]
fn gap_d_case_all_arms_and_default_then_read_is_accepted() {
    // Every case arm plus a default assigns ⇒ definitely assigned ⇒ accepted.
    let (_o, e, ok) = run("module m (input logic [1:0] s, output logic [7:0] y);\n\
         always_comb begin automatic int slot; y='0;\n\
         case (s) 2'd0: slot=1; 2'd1: slot=2; default: slot=7; endcase\n\
         y[slot] = 1'b1; end endmodule\n");
    assert!(ok, "case all-arms+default must be accepted: {e}");
}

#[test]
fn gap_d_plain_write_before_read_still_accepted() {
    // Round-3 D_OK case 1 (top-level write-before-read) stays accepted.
    let (_o, e, ok) = run("module m (input logic [7:0] s, output logic [7:0] y);\n\
         always_comb begin automatic int a; a = s; y = a[7:0]; end endmodule\n");
    assert!(ok, "plain write-before-read regressed: {e}");
}

#[test]
fn gap_d_write_only_in_if_no_else_read_after_is_loud() {
    // Written only on the true path, read AFTER the if: the false path reads a
    // cross-execution value ⇒ static ≠ automatic ⇒ must stay loud.
    let (_o, e, ok) = run("module m (input logic [7:0] s, output logic [7:0] y);\n\
         always_comb begin automatic int slot; y='0;\n\
         if (s > 0) slot = 5; y[0] = (slot > 2); end endmodule\n");
    assert!(!ok, "write-only-in-if then read-after must be loud");
    assert!(e.contains("automatic"), "expected the automatic loud: {e}");
}

#[test]
fn gap_d_accumulator_self_read_is_loud() {
    // `slot = slot + k` reads its own prior value before any write this
    // execution ⇒ accumulator ⇒ loud (static persists, automatic resets).
    let (_o, _e, ok) = run("module m (output logic [7:0] y);\n\
         always_comb begin automatic int slot; y='0;\n\
         for (int k=0;k<4;k++) slot = slot + k; y = slot[7:0]; end endmodule\n");
    assert!(!ok, "self-accumulating automatic must be loud");
}

#[test]
fn gap_d_partial_write_before_whole_is_loud() {
    // A bit-select write before any whole write is a read-modify-write of the
    // reset value ⇒ loud.
    let (_o, _e, ok) = run("module m (output logic [7:0] y);\n\
         always_comb begin automatic int slot; y='0;\n\
         slot[0] = 1'b1; y = slot[7:0]; end endmodule\n");
    assert!(!ok, "partial write of the reset value must be loud");
}

#[test]
fn gap_d_first_iter_read_before_write_is_loud() {
    // Iteration 0 reads `slot` before this execution writes it ⇒ observes a
    // cross-execution value ⇒ loud (the loop-body entry state is unassigned).
    let (_o, _e, ok) = run("module m (output logic [7:0] y);\n\
         always_comb begin automatic int slot; y='0;\n\
         for (int k=0;k<4;k++) begin if (k>0) y[k] = (slot > 0); slot = k; end end endmodule\n");
    assert!(!ok, "first-iteration read-before-write must be loud");
}

#[test]
fn gap_d_fork_racy_read_is_loud() {
    // Fork branches run concurrently — a write in one branch does not provably
    // precede a read in another. Reading an unassigned automatic across fork
    // branches is racy (static≠automatic) ⇒ must be loud, never accepted.
    let (_o, _e, ok) = run("module m (output logic [7:0] y);\n\
         initial begin automatic int x; y='0;\n\
         fork x = 5; y = x[7:0]; join end endmodule\n");
    assert!(
        !ok,
        "racy fork read of an unassigned automatic must be loud"
    );
}

#[test]
fn gap_d_fork_read_after_preassign_is_accepted() {
    // If the automatic is already assigned BEFORE the fork, every branch read
    // sees a current-execution value regardless of interleaving ⇒ accepted.
    let (_o, e, ok) = run("module m (output logic [7:0] y);\n\
         initial begin automatic int x; x = 3; y='0;\n\
         fork y = x[7:0]; join end endmodule\n");
    assert!(
        ok,
        "fork read after a pre-fork assignment must be accepted: {e}"
    );
}

#[test]
fn gap_d_automatic_colliding_with_module_net_is_loud() {
    // Adversarial find: an `automatic` block-local whose name collides with a
    // module-scope net used to be ALIASED onto that net by the v1 flatten, which both
    // corrupted the shared net AND bypassed the definite-assignment gate.
    //
    // ⚠️ The aliasing is gone — a shadow earns its own `$blk$` net — so the COLLISION
    // is no longer a reason to refuse and this no longer says "collides". What still
    // refuses is the second half of the original hazard, which this design was built
    // to carry: `y = x + 1` READS `x` before its first write, and a `$blk$` net is one
    // static net per block rather than one per entry, so the `automatic` lifetime is
    // still unimplementable here. verilator runs it (`y=1`, module `x=50`).
    //
    // The property worth pinning is unchanged and is the one the report was about:
    // this must never be accepted silently. The reason is asserted separately from
    // the refusal so a future change to either is visible.
    let (_o, e, ok) = run("module top;\n\
         logic [7:0] a; integer y; integer x;\n\
         always @(*) begin automatic int x; y = x + 1; x = a; end\n\
         initial begin x = 8'd50; a = 8'd10; #1 $display(\"y=%0d\", y); end endmodule\n");
    assert!(
        !ok,
        "a read-before-write `automatic` block-local must be loud, not aliased"
    );
    assert!(
        e.contains("per-entry lifetime"),
        "expected the lifetime gate, not the retired collision gate: {e}"
    );
    assert!(
        !e.contains("collides"),
        "the collision is no longer a reason — the shadow has its own net: {e}"
    );
}

#[test]
fn gap_d_nested_block_read_before_write_is_loud() {
    // An automatic declared in a NESTED block (inside `if`) read before its write
    // must still be caught (hoist recurses into nested blocks; the analysis scans
    // the nested block's own statements from its entry state).
    let (_o, _e, ok) = run("module m (input logic [7:0] s, output logic [7:0] y);\n\
         always_comb begin y='0;\n\
         if (s > 0) begin automatic int x; y[0] = (x > 3); x = 5; end end endmodule\n");
    assert!(!ok, "nested-block automatic read-before-write must be loud");
}

// ───────────────────── B / package-scoped call ─────────────────────

#[test]
fn pkg_scoped_subroutine_call_now_supported() {
    // SUPERSEDED by round-7 (§4.5.111): `bp::f(s)` was a clean loud in round-4; it is
    // now SUPPORTED for a self-contained, straight-line package function (`f` reads
    // only its formal `x` + a literal). It parses, elaborates, and simulates — no
    // error, no parser desync cascade.
    let (_o, e, ok) = run(
        "package bp; function automatic logic [1:0] f(input logic [1:0] x);\n\
         return x ^ 2'b01; endfunction endpackage\n\
         module m (input logic [1:0] s, output logic [1:0] y);\n\
         assign y = bp::f(s); endmodule\n",
    );
    assert!(ok, "self-contained package-scoped call now elaborates: {e}");
    assert_eq!(err_count(&e), 0, "no error expected: {e}");
    assert!(
        !e.contains("expected module item"),
        "must not desync into a parser cascade: {e}"
    );
}

#[test]
fn pkg_scoped_value_reference_still_works() {
    // A scoped VALUE reference `p::W` (no `(`) is unaffected by the call-guard.
    let (o, e, ok) = run("package p; localparam int W = 7; endpackage\n\
         module m (output logic [31:0] o); assign o = p::W; endmodule\n\
         module top; logic [31:0] o; m d(.o(o));\n\
         initial begin #1; $display(\"W=%0d\", o); end endmodule\n");
    assert!(ok, "scoped value reference regressed: {e}");
    assert!(o.contains("W=7"), "expected W=7, got: {o}");
}

#[test]
fn imported_function_call_still_works() {
    // The working workaround the loud message points at: `import` + bare call.
    let (o, e, ok) = run(
        "package bp; function automatic logic [1:0] f(input logic [1:0] x);\n\
         return x ^ 2'b01; endfunction endpackage\n\
         module m import bp::*; (input logic [1:0] s, output logic [1:0] y);\n\
         assign y = f(s); endmodule\n\
         module top; logic [1:0] s, y; m d(.s(s),.y(y));\n\
         initial begin s=2'b10; #1; $display(\"y=%0d\", y); end endmodule\n",
    );
    assert!(ok, "imported bare-name call regressed: {e}");
    assert!(o.contains("y=3"), "expected y=3 (2'b10 ^ 2'b01), got: {o}");
}

#[test]
fn body_local_enum_typedef_now_supported_round5() {
    // SUPERSEDED by round-5 Gap B: a body-local enum typedef in a FUNCTION body is
    // now SUPPORTED (was a v1 cut → loud in round-4). Labels register as constants
    // scoped to the function ({A=0,B=1}), so `v == B` folds. Drive x=1 (==B) → 3.
    let (o, _e, ok) = run("module tb;\n\
         logic [1:0] s, y;\n\
         function automatic logic [1:0] f(input logic [1:0] x);\n\
           typedef enum logic [1:0] { A = 2'd0, B = 2'd1 } e;\n\
           e v; v = e'(x);\n\
           if (v == B) return 2'd3; else return 2'd0;\n\
         endfunction\n\
         assign y = f(s);\n\
         initial begin s = 2'd1; #1 $display(\"y=%0d\", y); #1 $finish; end endmodule\n");
    assert!(
        ok,
        "round-5: body-local enum in a function body is supported"
    );
    assert!(o.contains("y=3"), "v==B(1) → 3, got: {o}");
}
