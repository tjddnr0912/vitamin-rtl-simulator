use super::*;

// S15d (v8 SVA subset). `assert property(@(clk) a |-> b)` parses to a
// `Stmt::ConcurrentAssert`; `|->` is Overlap, `|=>` is NonOverlap.
#[test]
fn concurrent_assert_property_parses_overlap_and_nonoverlap() {
    let (su, errs) = p("module m;\ninitial assert property (@(posedge clk) a |-> b);\nendmodule");
    assert!(errs.is_empty(), "concurrent assertion must parse: {errs:?}");
    let su = su.unwrap();
    let m = first_module(&su);
    let ModuleItem::Proc(pb) = m
        .body
        .iter()
        .find(|i| matches!(i, ModuleItem::Proc(_)))
        .unwrap()
    else {
        unreachable!()
    };
    assert!(
        matches!(
            &*pb.body,
            Stmt::ConcurrentAssert {
                implication_kind: ImplicationKind::Overlap,
                ..
            }
        ),
        "expected ConcurrentAssert{{Overlap}}, got {:?}",
        pb.body
    );

    let (su2, errs2) = p("module m;\ninitial assert property (@(posedge clk) a |=> b);\nendmodule");
    assert!(errs2.is_empty(), "non-overlap must parse: {errs2:?}");
    let su2 = su2.unwrap();
    let m2 = first_module(&su2);
    let ModuleItem::Proc(pb2) = m2
        .body
        .iter()
        .find(|i| matches!(i, ModuleItem::Proc(_)))
        .unwrap()
    else {
        unreachable!()
    };
    assert!(
        matches!(
            &*pb2.body,
            Stmt::ConcurrentAssert {
                implication_kind: ImplicationKind::NonOverlap,
                ..
            }
        ),
        "expected ConcurrentAssert{{NonOverlap}}, got {:?}",
        pb2.body
    );
}

// S15e (SVA slice S4). Sequence antecedents: `##n` cycle-delay parses to
// `Sequence::Delay`, `[*n]` consecutive repetition to `Sequence::Repeat`.
#[test]
fn concurrent_assert_seq_delay_parses() {
    let (su, errs) =
        p("module m;\ninitial assert property (@(posedge clk) a ##1 b |-> c);\nendmodule");
    assert!(
        errs.is_empty(),
        "sequence-delay antecedent must parse: {errs:?}"
    );
    let su = su.unwrap();
    let m = first_module(&su);
    let ModuleItem::Proc(pb) = m
        .body
        .iter()
        .find(|i| matches!(i, ModuleItem::Proc(_)))
        .unwrap()
    else {
        unreachable!()
    };
    let Stmt::ConcurrentAssert {
        antecedent,
        implication_kind: ImplicationKind::Overlap,
        ..
    } = &*pb.body
    else {
        panic!("expected ConcurrentAssert(Overlap), got {:?}", pb.body)
    };
    assert!(
        matches!(
            antecedent,
            Sequence::Delay {
                min: 1,
                max: Some(1),
                ..
            }
        ),
        "expected Sequence::Delay{{1}}, got {antecedent:?}"
    );
}

#[test]
fn concurrent_assert_seq_repeat_parses() {
    let (su, errs) =
        p("module m;\ninitial assert property (@(posedge clk) a[*3] |-> b);\nendmodule");
    assert!(
        errs.is_empty(),
        "repetition antecedent must parse: {errs:?}"
    );
    let su = su.unwrap();
    let m = first_module(&su);
    let ModuleItem::Proc(pb) = m
        .body
        .iter()
        .find(|i| matches!(i, ModuleItem::Proc(_)))
        .unwrap()
    else {
        unreachable!()
    };
    let Stmt::ConcurrentAssert { antecedent, .. } = &*pb.body else {
        panic!("expected ConcurrentAssert, got {:?}", pb.body)
    };
    assert!(
        matches!(
            antecedent,
            Sequence::Repeat {
                min: 3,
                max: Some(3),
                ..
            }
        ),
        "expected Sequence::Repeat{{3}}, got {antecedent:?}"
    );
}

// S15f (S5). Bounded ranges `##[m:n]` / `[*m:n]` now PARSE to Delay/Repeat
// with min != max; unbounded (`$`), `throughout`, `within` stay LOUD.
#[test]
fn concurrent_assert_seq_ranges_parse() {
    let (su, errs) =
        p("module m;\ninitial assert property (@(posedge clk) a ##[1:2] b |-> c);\nendmodule");
    assert!(errs.is_empty(), "bounded delay range must parse: {errs:?}");
    let su = su.unwrap();
    let m = first_module(&su);
    let ModuleItem::Proc(pb) = m
        .body
        .iter()
        .find(|i| matches!(i, ModuleItem::Proc(_)))
        .unwrap()
    else {
        unreachable!()
    };
    let Stmt::ConcurrentAssert { antecedent, .. } = &*pb.body else {
        panic!("expected ConcurrentAssert, got {:?}", pb.body)
    };
    assert!(
        matches!(
            antecedent,
            Sequence::Delay {
                min: 1,
                max: Some(2),
                ..
            }
        ),
        "expected Sequence::Delay{{1,2}}, got {antecedent:?}"
    );

    let (su2, errs2) =
        p("module m;\ninitial assert property (@(posedge clk) a[*2:3] |-> b);\nendmodule");
    assert!(
        errs2.is_empty(),
        "bounded repeat range must parse: {errs2:?}"
    );
    let m2 = first_module(su2.as_ref().unwrap());
    let ModuleItem::Proc(pb2) = m2
        .body
        .iter()
        .find(|i| matches!(i, ModuleItem::Proc(_)))
        .unwrap()
    else {
        unreachable!()
    };
    let Stmt::ConcurrentAssert { antecedent, .. } = &*pb2.body else {
        panic!("expected ConcurrentAssert, got {:?}", pb2.body)
    };
    assert!(
        matches!(
            antecedent,
            Sequence::Repeat {
                min: 2,
                max: Some(3),
                ..
            }
        ),
        "expected Sequence::Repeat{{2,3}}, got {antecedent:?}"
    );
}

// S15g (S6). Unbounded cycle delay `##[m:$]` parses to Delay{min, max:None}.
#[test]
fn concurrent_assert_seq_unbounded_delay_parses() {
    let (su, errs) =
        p("module m;\ninitial assert property (@(posedge clk) a ##[1:$] b |-> c);\nendmodule");
    assert!(errs.is_empty(), "unbounded delay must parse: {errs:?}");
    let m = first_module(su.as_ref().unwrap());
    let ModuleItem::Proc(pb) = m
        .body
        .iter()
        .find(|i| matches!(i, ModuleItem::Proc(_)))
        .unwrap()
    else {
        unreachable!()
    };
    let Stmt::ConcurrentAssert { antecedent, .. } = &*pb.body else {
        panic!("expected ConcurrentAssert, got {:?}", pb.body)
    };
    assert!(
        matches!(
            antecedent,
            Sequence::Delay {
                min: 1,
                max: None,
                ..
            }
        ),
        "expected Sequence::Delay{{1, $}}, got {antecedent:?}"
    );
}

// S15h (S7). `cond throughout seq` parses to Sequence::Throughout.
#[test]
fn concurrent_assert_throughout_parses() {
    let (su, errs) = p(
            "module m;\ninitial assert property (@(posedge clk) g throughout a ##2 c |-> d);\nendmodule",
        );
    assert!(errs.is_empty(), "throughout must parse: {errs:?}");
    let m = first_module(su.as_ref().unwrap());
    let ModuleItem::Proc(pb) = m
        .body
        .iter()
        .find(|i| matches!(i, ModuleItem::Proc(_)))
        .unwrap()
    else {
        unreachable!()
    };
    let Stmt::ConcurrentAssert { antecedent, .. } = &*pb.body else {
        panic!("expected ConcurrentAssert, got {:?}", pb.body)
    };
    // `g throughout (a ##2 c)` — throughout is looser than `##`.
    let Sequence::Throughout { seq, .. } = antecedent else {
        panic!("expected Sequence::Throughout, got {antecedent:?}")
    };
    assert!(
        matches!(&**seq, Sequence::Delay { min: 2, .. }),
        "throughout RHS must be the `a ##2 c` sequence, got {seq:?}"
    );
}

// S15i (S8). `b[->n]` goto / `b[=n]` nonconsec parse to Repeat with the right
// RepeatKind.
#[test]
fn concurrent_assert_goto_nonconsec_parse() {
    for (src, want_goto) in [
        (
            "module m;\ninitial assert property (@(posedge clk) a ##1 b[->2] |-> c);\nendmodule",
            true,
        ),
        (
            "module m;\ninitial assert property (@(posedge clk) a ##1 b[=2] |-> c);\nendmodule",
            false,
        ),
    ] {
        let (su, errs) = p(src);
        assert!(
            errs.is_empty(),
            "goto/nonconsec must parse: {errs:?} ({src})"
        );
        let m = first_module(su.as_ref().unwrap());
        let ModuleItem::Proc(pb) = m
            .body
            .iter()
            .find(|i| matches!(i, ModuleItem::Proc(_)))
            .unwrap()
        else {
            unreachable!()
        };
        let Stmt::ConcurrentAssert { antecedent, .. } = &*pb.body else {
            panic!("expected ConcurrentAssert")
        };
        // antecedent is `a ##1 b[->2]` = Delay{.., rhs: Repeat{kind}}.
        let Sequence::Delay { rhs, .. } = antecedent else {
            panic!("expected Delay, got {antecedent:?}")
        };
        let Sequence::Repeat { kind, min: 2, .. } = &**rhs else {
            panic!("expected Repeat with count 2, got {rhs:?}")
        };
        let is_goto = matches!(kind, RepeatKind::Goto);
        assert_eq!(is_goto, want_goto, "wrong repeat kind for {src}");
    }
}

// S15j (S9). `seq1 within seq2` parses to Sequence::Within (binary over `##`
// chains: `a within b ##2 c` = `a within (b ##2 c)`).
#[test]
fn concurrent_assert_within_parses() {
    let (su, errs) = p(
        "module m;\ninitial assert property (@(posedge clk) a within b ##2 c |-> d);\nendmodule",
    );
    assert!(errs.is_empty(), "within must parse: {errs:?}");
    let m = first_module(su.as_ref().unwrap());
    let ModuleItem::Proc(pb) = m
        .body
        .iter()
        .find(|i| matches!(i, ModuleItem::Proc(_)))
        .unwrap()
    else {
        unreachable!()
    };
    let Stmt::ConcurrentAssert { antecedent, .. } = &*pb.body else {
        panic!("expected ConcurrentAssert")
    };
    let Sequence::Within { seq2, .. } = antecedent else {
        panic!("expected Sequence::Within, got {antecedent:?}")
    };
    assert!(
        matches!(&**seq2, Sequence::Delay { min: 2, .. }),
        "within RHS must be `b ##2 c`, got {seq2:?}"
    );
}

// S13. Unbounded consecutive repeat `a[*m:$]` (m>=1) parses to
// `Sequence::Repeat { min: m, max: None, kind: Consec }`.
#[test]
fn concurrent_assert_consec_unbounded_parses() {
    let (su, errs) =
        p("module m;\ninitial assert property (@(posedge clk) a[*2:$] |-> b);\nendmodule");
    assert!(errs.is_empty(), "`a[*2:$]` must parse: {errs:?}");
    let m = first_module(su.as_ref().unwrap());
    let ModuleItem::Proc(pb) = m
        .body
        .iter()
        .find(|i| matches!(i, ModuleItem::Proc(_)))
        .unwrap()
    else {
        unreachable!()
    };
    let Stmt::ConcurrentAssert { antecedent, .. } = &*pb.body else {
        panic!("expected ConcurrentAssert")
    };
    assert!(
        matches!(
            antecedent,
            Sequence::Repeat {
                min: 2,
                max: None,
                kind: RepeatKind::Consec,
                ..
            }
        ),
        "expected Repeat{{2, None, Consec}}, got {antecedent:?}"
    );
}

// goto/nonconsec RANGES stay parser-LOUD (single counts only). Empty-match
// repetition `[*0:..]` now PARSES (2026-06-25) — a leading/standalone empty
// is honest-loud at ELABORATE instead (see cli `sva_empty_match.rs`), so it
// no longer belongs in this parser-level rejection net.
#[test]
fn concurrent_assert_deferred_seq_forms_are_loud() {
    for src in [
        "module m;\ninitial assert property (@(posedge clk) a ##1 b[->1:2] |-> c);\nendmodule",
        "module m;\ninitial assert property (@(posedge clk) a ##1 b[=1:2] |-> c);\nendmodule",
    ] {
        let (_, errs) = p(src);
        assert!(
            !errs.is_empty(),
            "deferred sequence form must be loud: {src}"
        );
    }
}

// Empty-match repetition now parses cleanly (the loud-ness moved to elaborate).
#[test]
fn empty_match_repetition_parses() {
    for src in [
        "module m;\ninitial assert property (@(posedge clk) a ##1 b[*0:$] |-> c);\nendmodule",
        "module m;\ninitial assert property (@(posedge clk) a ##1 b[*0:2] |-> c);\nendmodule",
        "module m;\ninitial assert property (@(posedge clk) a ##1 b[*] |-> c);\nendmodule",
        "module m;\ninitial assert property (@(posedge clk) a ##1 b[*0] |-> c);\nendmodule",
    ] {
        let (_, errs) = p(src);
        assert!(errs.is_empty(), "empty-match must parse: {src} -> {errs:?}");
    }
}
