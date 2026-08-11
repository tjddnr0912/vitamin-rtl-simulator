//! S0 (doc-21 §5/§7.3) — the ③층 design-level eligibility gate.
//!
//! Two properties are pinned here. (1) Each reject FAMILY actually fires — a
//! gate that never fires is vacuous (the `@(*)` lesson: the test written to
//! hold a line must exercise the shape that breaks it). (2) The P6 corpus
//! eligibility is an exact pinned count, so a future edit that silently widens
//! or narrows the gate moves a number a human must re-justify.
//!
//! The COMPLETENESS of the classification (every `SimOpts` sidecar assigned to
//! core or reject) is not a test — it is the compile-time exhaustive
//! destructure inside `design_eligibility` itself.

mod common;

use common::{build, corpus};
use sim_engine::native::design_eligibility;
use sim_engine::SimOpts;

fn reasons(src: &str) -> (bool, Vec<(&'static str, u32)>) {
    let ir = build(src);
    let e = design_eligibility(&ir, &SimOpts::default());
    (e.eligible, e.reject_reasons.into_iter().collect())
}

/// Plain RTL + a delay/wait testbench is the v1 TARGET, not a reject: tier-3
/// compiles suspension as a state machine (doc-21 §4.2), so `#delay` must not
/// disqualify — every real design's TB has one.
#[test]
fn plain_rtl_with_a_delay_tb_is_eligible() {
    let (ok, rs) = reasons(
        "module t;\n\
           reg clk = 0; reg [7:0] q = 0;\n\
           always #1 clk = ~clk;\n\
           always @(posedge clk) q <= q + 1;\n\
           initial begin #10 $display(\"q=%0d\", q); $finish; end\n\
         endmodule\n",
    );
    assert!(ok, "delay/wait TB must stay eligible: {rs:?}");
    assert!(rs.is_empty());
}

/// Frame calls are CORE by the revision-4 amendment (S3 absorbs T1/T2): a
/// design whose work lives in `automatic` subroutines is exactly the shape
/// tier-3 exists for (`bench/keccak` 호출형), so the gate must keep it.
#[test]
fn frame_calls_are_core_not_reject() {
    let (ok, rs) = reasons(
        "module t;\n\
           function automatic int add1(input int x); return x + 1; endfunction\n\
           int v = 0;\n\
           initial begin v = add1(v); $display(\"v=%0d\", v); $finish; end\n\
         endmodule\n",
    );
    assert!(ok, "framed user calls must stay eligible: {rs:?}");
}

/// Heap-storage kinds disqualify via the NET TABLE, not a sidecar — a plain
/// `int q[$]` has no sidecar entry at all, so the net-kind scan is the only
/// complete detector (the reason this gate reads the IR and not just opts).
#[test]
fn heap_storage_kinds_reject_by_net_kind() {
    let (ok, rs) = reasons(
        "module t;\n\
           string s;\n\
           int q[$];\n\
           int d[];\n\
           int aa[int];\n\
           initial begin s = \"x\"; q.push_back(1); d = new[2]; aa[3] = 4;\n\
             $display(\"%s %0d %0d %0d\", s, q[0], d[0], aa[3]); $finish; end\n\
         endmodule\n",
    );
    assert!(!ok);
    // V1 slice 2a admitted `dyn_array` and 2b admitted `string`: their values
    // live in `SimState::dyn_heap`, and tier-3 reaches them through the
    // composite reader, `write_routed`, and `HeapRouted` on the format path — so
    // the container is no longer a disqualifier. The two kinds with no
    // differential behind them yet stay, each under its own key (2c/2d).
    assert_eq!(
        rs,
        vec![("assoc", 1), ("queue", 1)],
        "each storage family must be counted under its own key"
    );
}

/// V1 slice 2a: a DYNAMIC ARRAY alone no longer disqualifies, and its two
/// ELEMENT refinements still do.
///
/// Written as its own case rather than as a deletion from the vector above: what
/// has to be asserted is the positive. And the refinements matter more than the
/// container did — while `dyn_array` refused every such net they were "subsumed"
/// and bound to `_`, so opening one row would have opened three. A `real r[]`
/// element store is an f64 lane and a `string s[]` element store routes through
/// the string heap; tier-3 has run neither.
#[test]
fn a_plain_dynamic_array_is_core_and_its_element_refinements_are_not() {
    let (ok, rs) = reasons(
        "module t;\n\
           int d[];\n\
           initial begin d = new[2]; d[0] = 7;\n\
             $display(\"%0d %0d\", d[0], d.size()); $finish; end\n\
         endmodule\n",
    );
    assert!(ok, "a plain `int d[]` must be eligible now: {rs:?}");
    assert_eq!(rs, vec![]);

    let ir = build("module t; reg a = 0; initial begin a = 1; $finish; end endmodule\n");
    let mut o = SimOpts::default();
    o.real_elem_dyn_nets.insert(0);
    o.string_elem_dyn_nets.insert(0);
    let e = design_eligibility(&ir, &o);
    assert!(!e.eligible);
    assert_eq!(
        e.reject_reasons.into_iter().collect::<Vec<_>>(),
        vec![("dyn_elem_real", 1), ("dyn_elem_string", 1)],
        "the element refinements must refuse in their own words"
    );
}

/// Sidecar-borne families fire from `SimOpts` alone (the tables the engine
/// consumes are the single source — no re-derivation from source text).
#[test]
fn sidecar_families_reject_from_opts() {
    let ir = build("module t; reg a = 0; initial begin a = 1; $finish; end endmodule\n");
    let base = design_eligibility(&ir, &SimOpts::default());
    assert!(
        base.eligible,
        "baseline must be clean: {:?}",
        base.reject_reasons
    );

    let mut o = SimOpts::default();
    o.fork_modes.insert((0, 0), sim_engine::JoinMode::All);
    o.probed_nets.push(0);
    o.class_new_sites.insert(0, 0);
    o.file_directed_stmts.insert(0);
    o.clocking_inputs.insert(0);
    let e = design_eligibility(&ir, &o);
    assert!(!e.eligible);
    assert_eq!(
        e.reject_reasons.into_iter().collect::<Vec<_>>(),
        vec![
            ("class", 1),
            ("clocking", 1),
            ("file_directed", 1),
            ("fork", 1),
            ("probe", 1),
        ]
    );
}

/// V1 slice 1: the SVA sidecars are CORE, and populating them disqualifies
/// NOTHING.
///
/// The companion of the test above, and written as its own case rather than as a
/// deletion from it: `assert_fire`/`assert_ctl` used to be listed there, so a
/// reader diffing that vector sees only that a row left. What has to be asserted
/// is the positive — these two tables can be non-empty on an ELIGIBLE design,
/// because both are consumed inside the shared `builtins::dispatch` that tier-3
/// already routes through, not by any machinery tier-3 lacks.
///
/// Populated ALONE (not alongside the reject families above): a verdict that
/// already has five reasons cannot show that a sixth stopped firing.
#[test]
fn sva_sidecars_are_core_and_reject_nothing() {
    let ir = build("module t; reg a = 0; initial begin a = 1; $finish; end endmodule\n");
    let mut o = SimOpts::default();
    o.assert_fire.insert(0);
    o.assert_ctl.insert(0, 1);
    let e = design_eligibility(&ir, &o);
    assert!(
        e.eligible,
        "SVA sidecars must not disqualify: {:?}",
        e.reject_reasons
    );
    assert_eq!(e.reject_reasons.len(), 0);
    assert_eq!(e.refused, None, "and nothing downstream may refuse either");
}

/// Statement-level families: `force`/`release` and `disable` are ordinary `Stmt`
/// variants, so ONLY a scan of the statement arena finds them — no sidecar
/// reports them (a plain `force b = c;` leaves `assign_ranks` empty). v1 has no
/// force machinery at all, so a design carrying one must go to the existing
/// engine rather than run with every `force` silently doing nothing.
#[test]
fn statement_level_families_reject() {
    let (ok, rs) = reasons(
        "module t;\n\
           reg a = 0; wire b; reg c = 1;\n\
           assign b = a;\n\
           initial begin force b = c; #1 release b; #1 $finish; end\n\
         endmodule\n",
    );
    assert!(!ok);
    assert_eq!(rs, vec![("force_release", 2)], "one force + one release");

    // ⭐ `disable` splits, and the split is the whole point: a plain
    // `disable <named block>` is the break/continue idiom, which elaborate
    // lowers as a diagnostic-shaped marker plus a sibling `Goto` that does the
    // control flow — the engine runs the marker as `StmtEffect::Nop`. It needs
    // NOTHING from tier-3, so rejecting it would cost a whole design (tier-3 has
    // no body-level fallback) for a statement with no runtime effect.
    let (ok2, rs2) = reasons(
        "module t;\n\
           integer i; reg [7:0] acc;\n\
           initial begin\n\
             acc = 0;\n\
             for (i = 0; i < 8; i = i + 1) begin : blk\n\
               if (i == 4) disable blk;\n\
               acc = acc + i[7:0];\n\
             end\n\
             $display(\"%0d\", acc); $finish;\n\
           end\n\
         endmodule\n",
    );
    assert!(
        ok2,
        "the break/continue idiom needs no tier-3 machinery: {rs2:?}"
    );

    // `disable fork` DOES kill descendants — that one stays a reject, and it can
    // appear with no `fork` in the design, so the sidecars never report it.
    let (ok3, rs3) = reasons(
        "module t;\n\
           initial begin #1 disable fork; #1 $finish; end\n\
         endmodule\n",
    );
    assert!(!ok3);
    assert_eq!(rs3, vec![("disable_fork", 1)]);
}

/// EFFECTS THAT NEVER PASS THROUGH THE WRITE FUNNEL — the prerequisite the
/// `Kernel` trait makes structural: a seeded `$random`/`$dist_*` writes its seed
/// back, `$cast`/`$value$plusargs` write their destination, the file family
/// advances descriptor state, `$readmem*` fills a memory and `$sformat` writes a
/// packed target — all INSIDE the call. An executor that only reproduced
/// `write_lvalue` would run these and silently drop every one of their effects
/// (`r = $random(seed)` would repeat the same draw forever).
///
/// The verdict uses `sim_ir::rhs_is_stmt_effect`, the SAME predicate the tier-2
/// VM's compile gate consults — two spellings would let the backends disagree
/// about one statement.
#[test]
fn effects_outside_the_write_funnel_reject() {
    // rhs form: the seed write-back happens in the call.
    let (ok, rs) = reasons(
        "module t; reg [31:0] r; integer seed;\n\
           initial begin seed = 1; r = $random(seed); $display(\"%0d\", r); $finish; end\n\
         endmodule\n",
    );
    assert!(!ok);
    assert_eq!(rs, vec![("stmt_effect", 1)]);

    // `$value$plusargs` — the shape a real testbench uses (bench/keccak's TB).
    // WIRED (the first family member lifted): `k_value_plusargs` runs the
    // shared `exec::plusargs::effect`, so this design is ACCEPTED now — and
    // the row must not count it, or the verdict and the executor disagree.
    let (ok2, rs2) = reasons(
        "module t; integer n; reg ok;\n\
           initial begin ok = $value$plusargs(\"N=%d\", n); $display(\"%0d\", n); $finish; end\n\
         endmodule\n",
    );
    assert!(ok2, "the wired member must be admitted, got {rs2:?}");
    assert_eq!(rs2, vec![]);

    // SysTask form: `$readmem*` writes a memory net without a funnel write.
    let (ok3, rs3) = reasons(
        "module t; reg [7:0] m [0:3];\n\
           initial begin $readmemh(\"x.hex\", m); $display(\"%0d\", m[0]); $finish; end\n\
         endmodule\n",
    );
    assert!(!ok3);
    assert_eq!(rs3, vec![("stmt_effect", 1)]);

    // `$sformat` and `$readmemb` — the other two net-writing task ids. Named by
    // this test's own doc but previously unreached (only `$readmemh` fired), so
    // two of the three were riding on the third.
    let (ok3b, rs3b) = reasons(
        "module t; reg [63:0] d;\n\
           initial begin $sformat(d, \"%0d\", 7); $display(\"%0d\", d); $finish; end\n\
         endmodule\n",
    );
    assert!(!ok3b);
    assert_eq!(rs3b, vec![("stmt_effect", 1)]);
    let (ok3c, rs3c) = reasons(
        "module t; reg [7:0] m [0:3];\n\
           initial begin $readmemb(\"x.bin\", m); $display(\"%0d\", m[0]); $finish; end\n\
         endmodule\n",
    );
    assert!(!ok3c);
    assert_eq!(rs3c, vec![("stmt_effect", 1)]);

    // The `$cast` TASK form: it writes its destination exactly as `$sformat`
    // does. A three-id `matches!` with an implicit catch-all accepted it —
    // measured eligible with an EMPTY reject map — which is why the predicate
    // now lives in sim-ir as an `_`-free match.
    let (ok3d, rs3d) = reasons(
        "module t; reg [7:0] d; reg [7:0] s;\n\
           initial begin s = 7; $cast(d, s); $display(\"%0d\", d); $finish; end\n\
         endmodule\n",
    );
    assert!(
        !ok3d,
        "the $cast TASK form writes its destination: {rs3d:?}"
    );
    assert_eq!(rs3d, vec![("stmt_effect", 1)]);

    // NEGATIVE, and it must be a SysFunc rhs — not a constant. A `q = 1` negative
    // cannot tell "the predicate says false for a PURE SysFunc" apart from "the
    // row fires on any SysFunc rhs at all".
    let (ok4, rs4) = reasons(
        "module t; reg [31:0] a, b, c;\n\
           initial begin\n\
             a = $clog2(33); b = $random(); c = $urandom();\n\
             $display(\"%0d %0d\", a, b ^ c); $finish;\n\
           end\n\
         endmodule\n",
    );
    assert!(
        ok4,
        "pure SysFunc rhs (incl. an UNSEEDED $random/$urandom) must stay eligible: {rs4:?}"
    );
}

/// The P6 corpus (the same 72 designs the P5 backend differential sweeps) is
/// ENTIRELY eligible: it generates plain-RTL processes by construction. Pinned
/// as an exact count — doc-21 §5 S0's corpus measurement, and teeth against a
/// silent widening/narrowing of the gate.
#[test]
fn p6_corpus_eligibility_is_72_of_72() {
    let mut eligible = 0usize;
    let mut total = 0usize;
    for d in corpus(0x5EED_F00D, 72) {
        total += 1;
        let ir = build(&d.src);
        if design_eligibility(&ir, &SimOpts::default()).eligible {
            eligible += 1;
        }
    }
    assert_eq!(total, 72);
    assert_eq!(eligible, 72, "the P6 corpus is plain RTL by construction");
}

/// The RUNTIME gate is the AND of the two halves — pinned on shapes that
/// actually reach EVERY arm.
///
/// The corpus alone cannot test this: measured, all 72 designs are
/// `eligible ∧ buildable`, so a gate hard-wired to `Ok(())` would pass. And the
/// `buildable`-vs-`build` comparison is a tautology while `build` delegates —
/// unless it is made on a REFUSED shape, where it becomes the guard that
/// catches the delegation being dropped (a `build` that stopped calling
/// `buildable` would happily lay out a design its own predicate refuses).
#[test]
fn the_runtime_gate_is_exactly_design_and_storage() {
    use sim_engine::native::arena::NetArena;

    let clean = "module t; reg [7:0] q = 0;\n\
         initial begin #1 q = 1; $display(\"q=%0d\", q); $finish; end endmodule\n";
    // Design gate refuses (heap kinds), storage would too.
    let design_refused = "module t; string s; int q[$];\n\
         initial begin s = \"a\"; q.push_back(1); $display(\"%s\", s); $finish; end endmodule\n";
    // Design gate PASSES (calls are core), storage refuses.
    //
    // ⚠️ The SHAPE here changed with S3a. It used to be a plain
    // `function automatic integer inc(input integer x)` whose body touched only
    // its own frame — which is precisely the subset S3a admits, so that design
    // now BUILDS and this arm would have gone vacuous. The refusal that is left
    // is a subroutine reading a MODULE net (`g`), which is the one thing the
    // delegation to the engine's frame executor cannot serve: that read would
    // come from the flat store a native run never writes.
    let storage_refused = "module t;\n\
           integer g;\n\
           function automatic integer addg(input integer x);\n\
             begin addg = x + g; end\n\
           endfunction\n\
           integer r;\n\
           initial begin g = 5; r = addg(3); $display(\"r=%0d\", r); $finish; end\n\
         endmodule\n";

    let mut saw_clean = 0;
    let mut saw_design_refused = 0;
    let mut saw_storage_refused = 0;
    for (name, src, want) in [
        ("clean", clean, None),
        ("design", design_refused, Some("queue")),
        (
            "storage",
            storage_refused,
            Some("a subroutine that names a net outside its own frame: S3b"),
        ),
    ] {
        let ir = build(src);
        // The storage-refused shape needs the REAL sidecars: with an empty
        // `func_table` the engine has no frame table and the design looks flat.
        let opts = sidecar_opts(src);
        let e = design_eligibility(&ir, &opts);
        let storage = NetArena::buildable(&ir, &opts);
        let gate = sim_engine::native::runtime_gate(&ir, &opts);
        assert_eq!(
            gate.err(),
            want,
            "{name}: runtime gate reason (eligible={}, storage={storage:?})",
            e.eligible
        );
        assert_eq!(gate_ok(&e), gate_expected(&e, &storage), "{name}");
        assert_eq!(e.buildable, storage.is_ok(), "{name}: buildable field");
        assert_eq!(e.refused, want, "{name}: reported reason");
        // Teeth for the delegation: on a refused shape the REAL build must
        // refuse too. This is the assertion that fails if `build` ever stops
        // calling `buildable`.
        assert_eq!(
            NetArena::build(&ir, &opts).is_ok(),
            storage.is_ok(),
            "{name}: `build` and `buildable` disagree"
        );
        match (e.eligible, storage.is_ok()) {
            (true, true) => saw_clean += 1,
            (false, _) => saw_design_refused += 1,
            (true, false) => saw_storage_refused += 1,
        }
    }
    // Non-vacuity: every arm was actually reached.
    assert_eq!(
        (saw_clean, saw_design_refused, saw_storage_refused),
        (1, 1, 1),
        "the three arms must each be exercised"
    );

    // And the property holds across the whole corpus (all clean, measured).
    for d in corpus(0x5EED_F00D, 72) {
        let ir = build(&d.src);
        let opts = SimOpts::default();
        let e = design_eligibility(&ir, &opts);
        assert!(
            e.eligible && e.buildable && e.refused.is_none(),
            "{}: corpus design unexpectedly refused: {:?}",
            d.name,
            e.refused
        );
        assert!(sim_engine::native::runtime_gate(&ir, &opts).is_ok());
    }
}

fn gate_ok(e: &sim_engine::native::NativeEligibility) -> bool {
    e.refused.is_none()
}

fn gate_expected(
    e: &sim_engine::native::NativeEligibility,
    storage: &Result<(), &'static str>,
) -> bool {
    e.eligible && storage.is_ok()
}

/// Elaborate WITH sidecars — `func_table` is what makes a subroutine design's
/// frame locals visible, and `SimOpts::default()` has none.
fn sidecar_opts(src: &str) -> SimOpts {
    struct Null;
    impl diag::LogSink for Null {
        fn emit(&self, _e: diag::LogEvent) {}
    }
    let (toks, _) = hdl_lexer::lex(src);
    let (su, _) = hdl_parser::parse(&toks, src);
    let sink = Null;
    let (_ir, sc) = elaborate::elaborate_with_timescale(
        &su.expect("unit"),
        &sink,
        &std::collections::BTreeMap::new(),
        -9,
    );
    SimOpts {
        two_state_nets: sc.two_state_nets,
        func_table: sc.func_table,
        ..SimOpts::default()
    }
}
