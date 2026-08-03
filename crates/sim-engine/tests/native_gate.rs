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
    assert_eq!(
        rs,
        vec![("assoc", 1), ("dyn_array", 1), ("queue", 1), ("string", 1)],
        "each storage family must be counted under its own key"
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
    o.assert_ctl.insert(0, 1);
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
            ("sva", 1),
        ]
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
