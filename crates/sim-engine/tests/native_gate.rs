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

/// V1 slice 2d: EVERY heap-storage kind is core, and the net-kind scan that
/// used to refuse them refuses nothing.
///
/// ⚠️ This test used to assert the opposite, under the name
/// `heap_storage_kinds_reject_by_net_kind`. Keeping it as a shrinking vector of
/// expected keys would have made it a bookkeeping exercise; the claim worth
/// pinning now is the POSITIVE one, and it is stronger than the four rows it
/// replaces: all four kinds in ONE design, so the funnel has to send four
/// destinations to three stores from a single call site.
///
/// The net-kind scan is still the right place to ask — a plain `int q[$]` has no
/// sidecar entry at all, which is why the gate reads the IR and not just opts.
/// What changed is the answer.
#[test]
fn every_heap_storage_kind_is_core() {
    let (ok, rs) = reasons(
        "module t;\n\
           string s;\n\
           int q[$];\n\
           int d[];\n\
           int aa[int];\n\
           int sa[string];\n\
           initial begin s = \"x\"; q.push_back(1); d = new[2]; aa[3] = 4; sa[\"k\"] = 5;\n\
             $display(\"%s %0d %0d %0d %0d\", s, q[0], d[0], aa[3], sa[\"k\"]); $finish; end\n\
         endmodule\n",
    );
    assert!(ok, "no heap kind may disqualify a design now: {rs:?}");
    assert!(rs.is_empty(), "no reject family may fire, got {rs:?}");

    // …and the storage half says the same thing, in its own words. Asked
    // separately because the two gates are independent and V0 measured that they
    // name the same feature twice — a design row opening while the storage row
    // still refuses buys exactly nothing.
    let ir = build(
        "module t; int aa[int]; initial begin aa[1] = 2; $display(\"%0d\", aa[1]); $finish; end endmodule\n",
    );
    assert_eq!(
        sim_engine::native::arena::NetArena::buildable(&ir, &SimOpts::default()).err(),
        None,
        "the storage gate must admit an assoc net too"
    );
}

/// V1 slice 2d's REJECT neighbour: a heap-kind chunk inside a CONCAT lvalue is
/// refused, by the STORAGE gate, in its own words.
///
/// ⚠️ This pin exists because nothing else can hold the line. The shape was
/// silently wrong from slice 2a onward — `write_routed` routes on a single-chunk
/// lvalue while the engine routes per CHUNK — and the only thing that surfaced
/// it was flipping the default backend and running the whole suite. Under the
/// normal default those two tests run on the VM, so a corpus differential never
/// reaches the arena at all, and the `assert_owns` instrument has nothing to
/// name. A refusal pin is what makes the row survive an edit.
///
/// A dyn element and an assoc element, because the router's bug was about the
/// CHUNK COUNT, not about which kinds are present.
///
/// ⚠️ NOT `string` — measured: `{s, x} = …` never reaches any gate, because
/// elaborate already refuses it ("a string / real value … inside a
/// concatenation lvalue is outside the v7 scope"). Listing it here would have
/// been a vacuous row asserting the storage gate does something a phase above
/// it already did.
#[test]
fn a_heap_chunk_inside_a_concat_lvalue_is_refused_by_storage() {
    use sim_engine::native::arena::NetArena;
    for src in [
        "module t; int d[]; logic [3:0] x;\n\
           initial begin d = new[2]; {d[0], x} = 8'hAB; $display(\"%0d %h\", d[0], x); end\n\
         endmodule\n",
        "module t; int aa[int]; logic [3:0] x;\n\
           initial begin {aa[5], x} = 8'hAB; $display(\"%0d %h\", aa[5], x); end\n\
         endmodule\n",
    ] {
        let ir = build(src);
        // The DESIGN gate says yes — heap kinds are core since slice 2. The
        // refusal is the storage gate's, which is the honest owner: the store
        // cannot represent a write it would have to split across two stores.
        assert!(
            design_eligibility(&ir, &SimOpts::default()).eligible,
            "the design gate has no quarrel with these: {src}"
        );
        assert_eq!(
            NetArena::buildable(&ir, &SimOpts::default()).err(),
            Some(
                "a heap-kind chunk inside a concat lvalue: the split would have to route per chunk"
            ),
            "storage must refuse: {src}"
        );
    }

    // The NEIGHBOUR that must keep running: one chunk is the supported shape,
    // and a multi-chunk lvalue of purely FLAT nets has nothing to route.
    for src in [
        "module t; int d[]; initial begin d = new[2]; d[0] = 7; $display(\"%0d\", d[0]); end endmodule\n",
        "module t; logic [3:0] a, b; initial begin {a, b} = 8'hAB; $display(\"%h%h\", a, b); end endmodule\n",
    ] {
        let ir = build(src);
        assert_eq!(
            NetArena::buildable(&ir, &SimOpts::default()).err(),
            None,
            "must still build: {src}"
        );
    }
}

/// V1 slice 2c: a QUEUE alone no longer disqualifies, and neither do the two
/// queue-OPERATION tables that used to ride the `queue_ops` row.
///
/// The positive assertion, for the same reason as the dyn-array case below it:
/// a deletion from the vector above proves the key is gone, not that the shape
/// runs. And this one carries a second claim the dyn-array case does not — that
/// `q[a:b]` (`queue_slice_stmts`) and `int bq[$:2]` (`queue_bounds`) are core
/// too, which is only true because both tables are read by code tier-3 already
/// shares and the slice's BOUND expressions were threaded through the reader.
#[test]
fn a_queue_with_its_operations_is_core() {
    let (ok, rs) = reasons(
        "module t;\n\
           int q[$];\n\
           int r[$];\n\
           int bq[$:2];\n\
           reg [7:0] a;\n\
           initial begin a = 8'd1;\n\
             q.push_back(a); q.push_back(2); q.push_back(3);\n\
             r = q[a:2]; bq.push_back(9);\n\
             $display(\"%0d %0d %0d\", q.size(), r.size(), bq.size()); $finish; end\n\
         endmodule\n",
    );
    assert!(ok, "a queue design must be design-eligible, got {rs:?}");
    assert!(rs.is_empty(), "no reject family may fire, got {rs:?}");

    // The opts half, populated ALONE on an otherwise-clean design: a verdict
    // that already has a reason cannot show that another stopped firing.
    let ir = build("module t; reg a = 0; initial begin a = 1; $finish; end endmodule\n");
    let mut o = SimOpts::default();
    o.queue_slice_stmts.insert(0);
    o.queue_bounds.insert(0, 2);
    let e = design_eligibility(&ir, &o);
    assert!(
        e.eligible,
        "the queue-op tables must disqualify nothing: {:?}",
        e.reject_reasons
    );
}

/// V1 slice 3b: a DYNAMIC ARRAY and BOTH of its element refinements are core.
///
/// ⚠️ This asserted the opposite until slice 3b. Slice 2a deliberately left
/// `real r[]` and `string s[]` refused — an f64 element lane and a byte-string
/// element store are not the same thing as a bit-vector element, and neither had
/// run. Measured since: both lanes live entirely in `SimState`'s heap methods
/// (`coerce_dyn_elem`, `alloc_dyn_array`, `dyn_read`/`dyn_write`), which slice 2
/// already routes every heap access to — so the refinements were as conservative
/// as the container had been.
///
/// The positive is asserted on SOURCE (all three shapes in one design) and on
/// OPTS (the two tables populated alone), because the tables are what the engine
/// consumes and a source-only case would not notice a row keyed on them.
#[test]
fn a_dynamic_array_and_both_element_refinements_are_core() {
    let (ok, rs) = reasons(
        "module t;\n\
           int d[];\n\
           string sd[];\n\
           real rd[];\n\
           initial begin d = new[2]; d[0] = 7; sd = new[2]; sd[0] = \"ab\"; rd = new[2]; rd[0] = 1.5;\n\
             $display(\"%0d %s %0f %0d\", d[0], sd[0], rd[0], d.size()); $finish; end\n\
         endmodule\n",
    );
    assert!(
        ok,
        "a dyn array with string/real elements must be eligible: {rs:?}"
    );
    assert_eq!(rs, vec![]);

    let ir = build("module t; reg a = 0; initial begin a = 1; $finish; end endmodule\n");
    let mut o = SimOpts::default();
    o.real_elem_dyn_nets.insert(0);
    o.string_elem_dyn_nets.insert(0);
    let e = design_eligibility(&ir, &o);
    assert!(
        e.eligible,
        "the element refinements must disqualify nothing: {:?}",
        e.reject_reasons
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
    o.file_directed_stmts.insert(0);
    o.clocking_inputs.insert(0);
    let e = design_eligibility(&ir, &o);
    assert!(!e.eligible);
    assert_eq!(
        e.reject_reasons.into_iter().collect::<Vec<_>>(),
        vec![
            ("clocking", 1),
            ("file_directed", 1),
            ("fork", 1),
            ("probe", 1),
        ]
    );
}

/// A2-i: the plain-OOP sidecars are CORE — a class handle net is an ordinary
/// slot holding an object id and the fields live in `class_heap`, which both
/// kernels borrow.
///
/// ⚠️ Written as an EXACT empty-reasons assertion rather than `eligible`,
/// because the failure this guards against is the row coming back under a new
/// name. And every one of these tables is populated: the census found that all
/// 160 class designs carry `class_rand`/`class_vtable` (they are per-CLASS and
/// exist the moment a `rand` field or a method is declared), so a rule keyed on
/// the TABLE rather than on the SITE would refuse all of them.
/// A7: functional COVERAGE is core — a covergroup is desugared, not executed.
///
/// ⚠️ The positive assertion is not the whole claim, and the neighbour below is
/// the half that matters: the summary is only correct because `simulate`
/// harvests the hit bitmaps from the store that RAN. Without that a native run
/// publishes `coverage_pct: 0.0` — a legal value, at exit 0 — so this row could
/// not be lifted on the desugaring argument alone.
#[test]
fn functional_coverage_is_core() {
    let ir = build("module t; reg a = 0; initial begin a = 1; $finish; end endmodule\n");
    let mut o = SimOpts::default();
    o.coverage_manifest.push(sim_engine::CovgInstMeta {
        inst: "t.c".to_string(),
        items: Vec::new(),
    });
    assert_eq!(
        design_eligibility(&ir, &o)
            .reject_reasons
            .into_iter()
            .collect::<Vec<_>>(),
        vec![],
        "a coverage manifest must disqualify nothing"
    );

    let (ok, rs) = reasons(
        "module t;\n\
           reg [1:0] x;\n\
           covergroup cg; cp: coverpoint x; endgroup\n\
           cg c = new;\n\
           initial begin x=0; c.sample(); x=3; c.sample(); $display(\"d\"); end\n\
         endmodule\n",
    );
    assert!(ok, "a covergroup design must be eligible: {rs:?}");
    assert!(rs.is_empty(), "no reject family may fire, got {rs:?}");
}

/// A8-a: a WHOLE-HANDLE COPY (`d2 = d1`, IEEE §7.10) is core.
///
/// The positive assertion, for the same reason the heap-kind test above is one:
/// a deleted key proves the row is gone, not that the shape runs. What makes it
/// core is that the whole feature lives in `SimState::dyn_heap` — one object
/// both kernels borrow, keyed by net id — and the two net ids come from the
/// sidecar rather than from an evaluation, so no net value is read on any path.
///
/// ⚠️ Asserted through `SimOpts` AND through source, because the two reach the
/// row differently: the table is what the gate counted, and the source is what
/// proves elaborate still emits it (a lowering change that stopped emitting the
/// marker would leave this row green while `d2 = d1` silently did nothing).
#[test]
fn a_whole_handle_copy_is_core() {
    let ir = build("module t; reg a = 0; initial begin a = 1; $finish; end endmodule\n");
    let mut o = SimOpts::default();
    o.handle_copy_stmts.insert(0, (0, 0));
    assert_eq!(
        design_eligibility(&ir, &o)
            .reject_reasons
            .into_iter()
            .collect::<Vec<_>>(),
        vec![],
        "a handle-copy marker must disqualify nothing"
    );

    let (ok, rs) = reasons(
        "module t;\n\
           int d1[], d2[];\n\
           initial begin d1 = new[2]; d1[0] = 7; d2 = d1; d1[0] = 9;\n\
             $display(\"%0d %0d\", d2[0], d1[0]); $finish; end\n\
         endmodule\n",
    );
    assert!(ok, "a design doing a handle copy must be eligible: {rs:?}");
    assert!(rs.is_empty(), "no reject family may fire, got {rs:?}");
}

#[test]
fn plain_oop_sidecars_are_core() {
    let ir = build("module t; reg a = 0; initial begin a = 1; $finish; end endmodule\n");
    let mut o = SimOpts::default();
    o.class_handle_nets.insert(0);
    o.class_new_sites.insert(0, 0);
    o.class_layouts.push(Default::default());
    o.class_field_inits.insert(0, Vec::new());
    o.class_rand.push(Vec::new());
    o.class_constraints.push(Vec::new());
    o.class_dist.push(Vec::new());
    o.class_randc.push(Vec::new());
    o.class_vtable.push(Vec::new());
    o.class_field_widths.insert(0, (8, false));
    // Both call-site shapes. A NON-virtual one dispatches statically; a VIRTUAL
    // one (`Some(vslot)`) is answered by `resolve_virtual_call`, which reads the
    // receiver handle's already-evaluated VALUE plus two shared tables and no
    // net at all — measured, not assumed, by
    // `a_virtual_call_dispatches_dynamically_on_tier_3`.
    o.class_calls.insert(0, (None, 3));
    o.class_calls.insert(1, (Some(0), 4));
    let e = design_eligibility(&ir, &o);
    assert_eq!(
        e.reject_reasons.into_iter().collect::<Vec<_>>(),
        vec![],
        "plain OOP must disqualify nothing"
    );
}

/// A2-i: `randomize()` is the OTHER half, and it is refused by the STATEMENT
/// rather than by the `class_rand` table — the table is per-class and present
/// in a design that never randomizes anything.
#[test]
fn a_randomize_call_is_refused_and_a_rand_field_alone_is_not() {
    let (ok, rs) = reasons(
        "module t;\n\
           class C; rand int unsigned v; constraint c { v < 10; } endclass\n\
           C o; int r;\n\
           initial begin o = new(); r = o.randomize(); $display(\"%0d\", o.v); $finish; end\n\
         endmodule\n",
    );
    assert!(!ok, "a randomize() call must be refused");
    assert!(
        rs.iter().any(|&(k, _)| k == "class_crv"),
        "the refusal must name the CRV row: {rs:?}"
    );

    // …while the same class WITHOUT the call disqualifies nothing, even though
    // elaborate still emits `class_rand`/`class_constraints` for it.
    let (ok2, rs2) = reasons(
        "module t;\n\
           class C; rand int unsigned v; constraint c { v < 10; } endclass\n\
           C o;\n\
           initial begin o = new(); o.v = 4; $display(\"%0d\", o.v); $finish; end\n\
         endmodule\n",
    );
    assert!(
        ok2,
        "a rand field with no randomize() must stay core: {rs2:?}"
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
    // A1-iv-a: `$sscanf` is WIRED (it scans a string, so it needs no file
    // table); its seven fd siblings are not, which is what the negative below
    // still holds.
    let (oks, rss) = reasons(
        "module t; string s; int a, n;\n\
           initial begin s = \"7\"; n = $sscanf(s, \"%d\", a); $display(\"%0d %0d\", n, a); $finish; end\n\
         endmodule\n",
    );
    assert!(oks, "`$sscanf` is wired and must be admitted: {rss:?}");
    assert_eq!(rss, vec![]);

    // A1-iv-b: six of the seven fd members are WIRED. All six, because the
    // carve-out is a list and a missing entry leaves that member refusing while
    // its `k_*` runs.
    for (what, body) in [
        ("$fopen/$fgetc", "integer fd, c;\n\
             initial begin fd = $fopen(\"x.txt\", \"r\"); c = $fgetc(fd); $display(\"%0d\", c); $finish; end"),
        ("$feof", "integer fd, e;\n\
             initial begin fd = $fopen(\"x.txt\", \"r\"); e = $feof(fd); $display(\"%0d\", e); $finish; end"),
        ("$ungetc", "integer fd, u;\n\
             initial begin fd = $fopen(\"x.txt\", \"r\"); u = $ungetc(65, fd); $display(\"%0d\", u); $finish; end"),
        ("$fgets", "integer fd, n; string s;\n\
             initial begin fd = $fopen(\"x.txt\", \"r\"); n = $fgets(s, fd); $display(\"%0d\", n); $finish; end"),
        ("$fscanf", "integer fd, n; int a;\n\
             initial begin fd = $fopen(\"x.txt\", \"r\"); n = $fscanf(fd, \"%d\", a); $display(\"%0d\", n); $finish; end"),
    ] {
        let (okf, rsf) = reasons(&format!("module t; {body}\nendmodule\n"));
        assert!(okf, "`{what}` is wired and must be admitted: {rsf:?}");
        assert_eq!(rsf, vec![], "{what}");
    }

    // A1-iv-c wired `$fread`, the last member — so this test has NO negative
    // half left, and that is the point rather than an omission. The row is not
    // dead code: it still fires for any effectful id added later, which is
    // exactly what `every_stmt_effect_family_member_is_wired` pins.
    let (okr, rsr) = reasons(
        "module t; integer fd, n; reg [7:0] m [0:3];\n\
           initial begin fd = $fopen(\"x.txt\", \"r\"); n = $fread(m, fd); $display(\"%0d\", n); $finish; end\n\
         endmodule\n",
    );
    assert!(okr, "`$fread` is wired and must be admitted: {rsr:?}");
    assert_eq!(rsr, vec![]);

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

    // A1-i: the SECOND wired member. `q.pop_front()`/`pop_back()` are
    // store-INDEPENDENT — the pop mutates `SimState::dyn_heap`, which is one
    // object shared by both backends, and it reads no net value — so tier-3
    // delegates and the row must stop counting them. Both ids, because a
    // carve-out written for one leaves the other refusing while its `k_*` runs.
    for pop in ["pop_front", "pop_back"] {
        let (okp, rsp) = reasons(&format!(
            "module t; int q[$]; int r;\n\
               initial begin q.push_back(3); r = q.{pop}(); $display(\"%0d\", r); $finish; end\n\
             endmodule\n"
        ));
        assert!(okp, "`q.{pop}()` is wired and must be admitted: {rsp:?}");
        assert_eq!(rsp, vec![]);
    }

    // A1-ii: the REF-ARG writers. Their bodies moved to `exec::stmt_effect`,
    // generic over `Kernel`, so the operand reads and the ref-arg write both
    // land in the calling kernel's store. All four, because the carve-out is a
    // list and a missing entry leaves that member refusing while its `k_*` runs.
    for (what, body) in [
        (
            "$random(seed)",
            "integer seed, r;\n\
               initial begin seed = 1; r = $random(seed); $display(\"%0d %0d\", r, seed); $finish; end",
        ),
        (
            "$dist_uniform(seed,..)",
            "integer seed, r;\n\
               initial begin seed = 1; r = $dist_uniform(seed, 0, 9); $display(\"%0d %0d\", r, seed); $finish; end",
        ),
        (
            "ok = $cast(dst, src)",
            "byte d; int s; int ok;\n\
               initial begin s = 5; ok = $cast(d, s); $display(\"%0d %0d\", ok, d); $finish; end",
        ),
        (
            "aa.first(k)",
            "int aa[int]; int k, st;\n\
               initial begin aa[3] = 1; st = aa.first(k); $display(\"%0d %0d\", st, k); $finish; end",
        ),
    ] {
        let (okw, rsw) = reasons(&format!("module t; {body}\nendmodule\n"));
        assert!(okw, "`{what}` is wired and must be admitted: {rsw:?}");
        assert_eq!(rsw, vec![], "{what}");
    }

    // A1-iii: the three FLAT-writing task ids are WIRED. `$sformat`,
    // `$readmemb/h` and the `$cast` TASK form collect their destination writes
    // through `TaskWrites::Collect` and the calling kernel applies them, so the
    // row must stop counting them. All four spellings, because the carve-out is
    // a `matches!` list and a missing arm leaves that id refusing.
    for (what, body) in [
        (
            "$readmemh",
            "reg [7:0] m [0:3];\n\
               initial begin $readmemh(\"x.hex\", m); $display(\"%0d\", m[0]); $finish; end",
        ),
        (
            "$readmemb",
            "reg [7:0] m [0:3];\n\
               initial begin $readmemb(\"x.bin\", m); $display(\"%0d\", m[0]); $finish; end",
        ),
        (
            "$sformat",
            "reg [63:0] d;\n\
               initial begin $sformat(d, \"%0d\", 7); $display(\"%0d\", d); $finish; end",
        ),
        (
            "$cast task form",
            "reg [7:0] d; reg [7:0] s;\n\
               initial begin s = 7; $cast(d, s); $display(\"%0d\", d); $finish; end",
        ),
    ] {
        let (okw, rsw) = reasons(&format!("module t; {body}\nendmodule\n"));
        assert!(okw, "`{what}` is wired and must be admitted: {rsw:?}");
        assert_eq!(rsw, vec![], "{what}");
    }

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
    // Design gate refuses, storage would too.
    //
    // ⚠️ The SHAPE here changed with V1 slice 2. It used to be `string s; int
    // q[$]`, and slices 2a/2b/2c admitted all three of those kinds — this arm
    // would have gone vacuous exactly as the storage arm nearly did at S3a.
    // `real` is the kind left that BOTH halves refuse under their own names
    // (design: the `real` row; storage: "real: S2 width class"), which is what
    // this arm needs — a kind only one half refused would not test the AND.
    let design_refused = "module t; real r;\n\
         initial begin r = 1.5; $display(\"%f\", r); $finish; end endmodule\n";
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
        ("design", design_refused, Some("real")),
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

/// A1 CLOSED: every member of the `stmt_effect` family is wired, so the row can
/// no longer fire on today's ids.
///
/// ⚠️ The row is NOT deleted, and that is deliberate. A new effectful
/// `SysFuncId`/`SysTaskId` added later would land in `sysfunc_is_stmt_effect` /
/// `systask_net_write` (both `_`-free, so it must be classified) and immediately
/// start refusing designs again — which is correct, because its `k_*` would not
/// be written yet. What this test pins is that the row is currently EMPTY, so a
/// future reader can tell "nothing left to wire" from "somebody deleted the
/// carve-out".
///
/// Driven from the ids themselves rather than from a hand list: `SysFuncId` has
/// no iterator, so the family is reached through DESIGNS, one per member, and a
/// member that stops being admitted names itself in the failure.
#[test]
fn every_stmt_effect_family_member_is_wired() {
    let cases: [(&str, &str); 15] = [
        ("$random(seed)", "integer s, r; initial begin s = 1; r = $random(s); end"),
        ("$dist_uniform", "integer s, r; initial begin s = 1; r = $dist_uniform(s, 0, 9); end"),
        ("$cast func", "byte d; int s; int ok; initial begin s = 5; ok = $cast(d, s); end"),
        ("$cast task", "reg [7:0] d, s; initial begin s = 7; $cast(d, s); end"),
        ("$value$plusargs", "integer n; reg ok; initial ok = $value$plusargs(\"N=%d\", n);"),
        ("q.pop_front", "int q[$]; int r; initial begin q.push_back(1); r = q.pop_front(); end"),
        ("q.pop_back", "int q[$]; int r; initial begin q.push_back(1); r = q.pop_back(); end"),
        ("aa.first", "int aa[int]; int k, st; initial begin aa[1] = 2; st = aa.first(k); end"),
        ("aa.next", "int aa[int]; int k, st; initial begin aa[1] = 2; st = aa.next(k); end"),
        ("$sformat", "reg [63:0] d; initial $sformat(d, \"%0d\", 7);"),
        ("$readmemh", "reg [7:0] m [0:3]; initial $readmemh(\"x.hex\", m);"),
        ("$fopen/$fgetc", "integer fd, c; initial begin fd = $fopen(\"x\", \"r\"); c = $fgetc(fd); end"),
        ("$feof/$ungetc", "integer fd, e, u; initial begin fd = $fopen(\"x\", \"r\"); e = $feof(fd); u = $ungetc(65, fd); end"),
        ("$fgets/$fscanf", "integer fd, n; string s; int a; initial begin fd = $fopen(\"x\", \"r\"); n = $fgets(s, fd); n = $fscanf(fd, \"%d\", a); end"),
        ("$fread", "integer fd, n; reg [7:0] m [0:3]; initial begin fd = $fopen(\"x\", \"r\"); n = $fread(m, fd); end"),
    ];
    for (what, body) in cases {
        let (ok, rs) = reasons(&format!("module t; {body}\nendmodule\n"));
        assert!(
            ok,
            "`{what}` must be admitted — the family is closed: {rs:?}"
        );
        assert!(
            !rs.iter().any(|(k, _)| *k == "stmt_effect"),
            "`{what}` still counts against the `stmt_effect` row: {rs:?}"
        );
    }
}
