use super::*;

// S1. initial begin: blocking + nonblocking mix
#[test]
fn s1_initial_blocking_nonblocking() {
    let pb = proc_of("initial begin a = 1; q <= d; end");
    assert_eq!(pb.kind, ProcKind::Initial);
    assert!(pb.sensitivity.is_none());
    let (_l, _d, stmts) = as_block(&pb.body);
    assert!(matches!(stmts[0], Stmt::Blocking { .. }));
    assert!(matches!(stmts[1], Stmt::NonBlocking { .. }));
}

// S2. always @(posedge clk) if/else (no begin) — sensitivity on the BLOCK
#[test]
fn s2_always_posedge_if_else() {
    let pb = proc_of("always @(posedge clk) if (rst) q <= 0; else q <= d;");
    assert_eq!(pb.kind, ProcKind::Always);
    let Some(Sensitivity::List(evs)) = &pb.sensitivity else {
        panic!()
    };
    assert_eq!(evs.len(), 1);
    assert_eq!(evs[0].edge, Edge::Posedge);
    let Stmt::If { else_s, .. } = &*pb.body else {
        panic!("body not If")
    };
    assert!(else_s.is_some());
}

// S3. posedge/negedge `or`-separated sensitivity list
#[test]
fn s3_sensitivity_or_list() {
    let pb = proc_of("always @(posedge clk or negedge rst_n) q <= d;");
    let Some(Sensitivity::List(evs)) = &pb.sensitivity else {
        panic!()
    };
    assert_eq!(evs.len(), 2);
    assert_eq!(evs[0].edge, Edge::Posedge);
    assert_eq!(evs[1].edge, Edge::Negedge);
}

// S4. always_comb + case: sensitivity MUST be None (@ never consumed); multi-label
#[test]
fn s4_always_comb_case() {
    let pb = proc_of("always_comb case (sel) 2'b00, 2'b01: y = a; default: y = b; endcase");
    assert_eq!(pb.kind, ProcKind::AlwaysComb);
    assert!(pb.sensitivity.is_none());
    let Stmt::Case { kind, items, .. } = &*pb.body else {
        panic!()
    };
    assert_eq!(*kind, CaseKind::Case);
    let CaseItem::Match { labels, .. } = &items[0] else {
        panic!()
    };
    assert_eq!(labels.len(), 2); // two labels share one body
    assert!(matches!(items[1], CaseItem::Default { .. }));
}

// S5. casez kind + `default` WITHOUT a colon
#[test]
fn s5_casez_default_no_colon() {
    let pb = proc_of("always_comb casez (req) 4'b1???: g = 1; default g = 0; endcase");
    let Stmt::Case { kind, items, .. } = &*pb.body else {
        panic!()
    };
    assert_eq!(*kind, CaseKind::Casez);
    assert!(matches!(items[1], CaseItem::Default { .. }));
}

// S6. for-loop — init/step are Blocking built WITHOUT consuming the ';'
#[test]
fn s6_for_loop() {
    let pb = proc_of("initial for (i = 0; i < 8; i = i + 1) sum = sum + i;");
    let Stmt::For {
        init, step, body, ..
    } = &*pb.body
    else {
        panic!()
    };
    assert!(matches!(**init, Stmt::Blocking { .. }));
    assert!(matches!(**step, Stmt::Blocking { .. }));
    assert!(matches!(**body, Stmt::Blocking { .. }));
}

// S7. while + $display systask call (name retains `$`, 2 args)
#[test]
fn s7_while_and_display() {
    let pb = proc_of("initial while (cnt < 8) begin $display(\"c=%d\", cnt); cnt = cnt + 1; end");
    let Stmt::While { body, .. } = &*pb.body else {
        panic!()
    };
    let (_l, _d, stmts) = as_block(body);
    let Stmt::SysTaskCall { name, args, .. } = &stmts[0] else {
        panic!()
    };
    assert_eq!(name.name, "$display");
    assert_eq!(args.len(), 2);
}

// S8. #delay statement with body, then $finish with NO parens (empty args)
#[test]
fn s8_delay_and_finish() {
    let pb = proc_of("initial begin #20 rst = 0; #200 $finish; end");
    let (_l, _d, stmts) = as_block(&pb.body);
    let Stmt::DelayCtrl { body: b0, .. } = &stmts[0] else {
        panic!()
    };
    assert!(matches!(b0.as_deref(), Some(Stmt::Blocking { .. })));
    let Stmt::DelayCtrl { body: b1, .. } = &stmts[1] else {
        panic!()
    };
    let Some(Stmt::SysTaskCall { name, args, .. }) = b1.as_deref() else {
        panic!()
    };
    assert_eq!(name.name, "$finish");
    assert!(args.is_empty());
}

// S9. dangling-else binds to the INNER if
#[test]
fn s9_dangling_else_inner() {
    let pb = proc_of("initial if (a) if (b) x = 1; else x = 2;");
    let Stmt::If { then_s, else_s, .. } = &*pb.body else {
        panic!()
    };
    assert!(else_s.is_none(), "outer if must NOT own the else");
    let Stmt::If {
        else_s: inner_else, ..
    } = &**then_s
    else {
        panic!("then not If")
    };
    assert!(inner_else.is_some(), "inner if owns the else");
}

// S10. named begin-end with a local decl + end-label (label consumed, no hang)
#[test]
fn s10_named_block_local_decl() {
    let pb = proc_of("initial begin : blk reg [7:0] tmp; tmp = a; end");
    let (label, decls, stmts) = as_block(&pb.body);
    assert_eq!(label.as_ref().unwrap().name, "blk");
    assert_eq!(decls.len(), 1);
    assert_eq!(stmts.len(), 1);
    assert!(matches!(stmts[0], Stmt::Blocking { .. }));
}

// S11. always @(*) Star + nested begin
#[test]
fn s11_nested_block_and_star() {
    let pb = proc_of("always @(*) begin a = b; begin c = d; end end");
    assert!(matches!(pb.sensitivity, Some(Sensitivity::Star)));
    let (_l, _d, stmts) = as_block(&pb.body);
    assert!(matches!(stmts[0], Stmt::Blocking { .. }));
    assert!(matches!(stmts[1], Stmt::Block { .. }));
}

// S12. recovery: garbage statement → Error, no infinite loop, following stmt parses
#[test]
fn s12_recovery_garbage_stmt() {
    let (su, errs) = p("module m;\ninitial begin & ; x = 1; end\nendmodule");
    assert!(!errs.is_empty(), "expected a recovered error");
    let su = su.unwrap();
    let m = first_module(&su);
    let Some(ModuleItem::Proc(pb)) = m.body.iter().find(|i| matches!(i, ModuleItem::Proc(_)))
    else {
        panic!("no proc block")
    };
    let (_l, _d, stmts) = as_block(&pb.body);
    assert!(
        stmts.iter().any(|s| matches!(s, Stmt::Error(_))),
        "garbage → Error"
    );
    assert!(
        stmts.iter().any(|s| matches!(s, Stmt::Blocking { .. })),
        "must recover and parse `x = 1;`"
    );
}

// S13. fork / join_none — JoinKind from an Ident token (not a keyword)
#[test]
fn s13_fork_join_none() {
    let pb = proc_of("initial fork #10 a = 1; #20 b = 1; join_none");
    let Stmt::Fork { stmts, join, .. } = &*pb.body else {
        panic!()
    };
    assert_eq!(*join, JoinKind::JoinNone);
    assert_eq!(stmts.len(), 2);
}

// S14. repeat body is a bare EventCtrl (body None); wait body None;
//      and intra-assign delay `q <= #1 d;` parses CLEAN into the delay field.
#[test]
fn s14_event_body_none_and_intra_delay() {
    let src = "module m;\ninitial begin repeat (8) @(posedge clk); wait (ready); q <= #1 d; end\nendmodule";
    let (su, errs) = p(src);
    assert!(errs.is_empty(), "intra-assign delay parses clean: {errs:?}");
    let su = su.unwrap();
    let m = first_module(&su);
    let Some(ModuleItem::Proc(pb)) = m.body.iter().find(|i| matches!(i, ModuleItem::Proc(_)))
    else {
        panic!("no proc block")
    };
    let (_l, _d, stmts) = as_block(&pb.body);
    let Stmt::Repeat { body, .. } = &stmts[0] else {
        panic!()
    };
    let Stmt::EventCtrl { body: eb, .. } = &**body else {
        panic!("repeat body not EventCtrl")
    };
    assert!(eb.is_none()); // `@(posedge clk);` → body None
    let Stmt::Wait { body: wb, .. } = &stmts[1] else {
        panic!()
    };
    assert!(wb.is_none());
    // intra-assign delay is CAPTURED into the AST delay field (the
    // elaborator decides semantics: blocking = real, NBA = loud defer).
    let Stmt::NonBlocking { delay, .. } = &stmts[2] else {
        panic!("not NonBlocking")
    };
    assert!(delay.is_some(), "intra-assign delay must be captured");
}

// S14b. blocking intra-assign delay `a = #3 b;` captures into `delay`; blocking
//       intra-assign EVENT control `a = @(ev) b` / `a = repeat(n) @(ev) b`
//       captures into `event` (slice: repeat-event intra-assignment).
#[test]
fn s14b_blocking_intra_delay_and_event_control_captured() {
    let (su, errs) = p("module m;\ninitial a = #3 b;\nendmodule");
    assert!(
        errs.is_empty(),
        "blocking intra-delay parses clean: {errs:?}"
    );
    let su = su.unwrap();
    let m = first_module(&su);
    let Some(ModuleItem::Proc(pb)) = m.body.iter().find(|i| matches!(i, ModuleItem::Proc(_)))
    else {
        panic!("no proc block")
    };
    let Stmt::Blocking { delay, event, .. } = &*pb.body else {
        panic!("not Blocking: {:?}", pb.body)
    };
    assert!(
        delay.is_some() && event.is_none(),
        "blocking intra-assign delay must be captured (no event)"
    );

    // Plain `@(ev)` intra-assign now parses clean and captures `event` (repeat=None).
    let (su, errs) = p("module m;\ninitial a = @(posedge clk) b;\nendmodule");
    assert!(
        errs.is_empty(),
        "intra-assign event control parses clean: {errs:?}"
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
    let Stmt::Blocking { event, delay, .. } = &*pb.body else {
        panic!("not Blocking")
    };
    let ev = event.as_ref().expect("event control must be captured");
    assert!(
        delay.is_none() && ev.repeat.is_none(),
        "plain @(ev): repeat None"
    );

    // `repeat(n) @(ev)` captures the count.
    let (su, errs) = p("module m;\ninitial a = repeat(3) @(posedge clk) b;\nendmodule");
    assert!(
        errs.is_empty(),
        "repeat-event intra-assign parses clean: {errs:?}"
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
    let Stmt::Blocking { event, .. } = &*pb.body else {
        panic!("not Blocking")
    };
    assert!(
        event.as_ref().and_then(|e| e.repeat.as_ref()).is_some(),
        "repeat(n) @(ev): repeat count must be captured"
    );
}

// S15. SV immediate assert (IEEE 1800 §16.3) desugars AT PARSE TIME to
//      `Stmt::If` — the frozen AST Stmt set (M7) gains no variant, and `if`
//      already has the exact assert condition semantics (X/Z → else). No
//      else clause ⇒ the IEEE default failure action is synthesized as
//      `$error("Assertion failed")`.
#[test]
fn s15_assert_desugars_to_if_with_default_error() {
    let (su, errs) = p("module m;\ninitial assert (a == 1);\nendmodule");
    assert!(errs.is_empty(), "immediate assert parses clean: {errs:?}");
    let su = su.unwrap();
    let m = first_module(&su);
    let Some(ModuleItem::Proc(pb)) = m.body.iter().find(|i| matches!(i, ModuleItem::Proc(_)))
    else {
        panic!("no proc block")
    };
    let Stmt::If { then_s, else_s, .. } = &*pb.body else {
        panic!("assert must desugar to If: {:?}", pb.body)
    };
    assert!(
        matches!(**then_s, Stmt::Null(_)),
        "no pass action → Null then-branch"
    );
    let Some(e) = else_s else {
        panic!("missing else clause must synthesize the default action")
    };
    let Stmt::SysTaskCall { name, args, .. } = &**e else {
        panic!("default else must be a $error call: {e:?}")
    };
    assert_eq!(name.name, "$error");
    assert_eq!(args.len(), 1);
    assert!(matches!(&args[0].kind, ExprKind::StrLit { raw } if raw.contains("Assertion failed")));
}

// S15b. explicit pass/else actions map onto the If branches verbatim; the
//       else-only form gets a Null then-branch.
#[test]
fn s15b_assert_actions_map_to_if_branches() {
    let (su, errs) =
        p("module m;\ninitial assert (a) $display(\"ok\"); else $display(\"no\");\nendmodule");
    assert!(errs.is_empty(), "{errs:?}");
    let su = su.unwrap();
    let m = first_module(&su);
    let Some(ModuleItem::Proc(pb)) = m.body.iter().find(|i| matches!(i, ModuleItem::Proc(_)))
    else {
        panic!("no proc block")
    };
    let Stmt::If { then_s, else_s, .. } = &*pb.body else {
        panic!("not If: {:?}", pb.body)
    };
    let Stmt::SysTaskCall { name, .. } = &**then_s else {
        panic!("pass action must be the then-branch")
    };
    assert_eq!(name.name, "$display");
    let Some(e) = else_s else { panic!("no else") };
    let Stmt::SysTaskCall { name, .. } = &**e else {
        panic!("user else action must be kept verbatim")
    };
    assert_eq!(name.name, "$display");

    // else-only: `assert (a) else x = 1;`
    let (su2, errs2) = p("module m;\ninitial assert (a) else x = 1;\nendmodule");
    assert!(errs2.is_empty(), "{errs2:?}");
    let su2 = su2.unwrap();
    let m2 = first_module(&su2);
    let Some(ModuleItem::Proc(pb2)) = m2.body.iter().find(|i| matches!(i, ModuleItem::Proc(_)))
    else {
        panic!("no proc block")
    };
    let Stmt::If { then_s, else_s, .. } = &*pb2.body else {
        panic!("not If")
    };
    assert!(matches!(**then_s, Stmt::Null(_)));
    assert!(matches!(else_s.as_deref(), Some(Stmt::Blocking { .. })));
}

// S15c. the DEFERRED forms now PARSE to `Stmt::DeferredAssert` (faithful
//       deferred-assert slice): `#0` = Observed, `final` = Reactive. A
//       non-zero `#<n>` delay on an assert stays a LOUD parse error.
#[test]
fn s15c_deferred_assert_parses_observed_and_reactive() {
    for (src, want) in [
        (
            "module m;\ninitial assert #0 (a);\nendmodule",
            AssertDefer::Observed,
        ),
        (
            "module m;\ninitial assert final (a);\nendmodule",
            AssertDefer::Reactive,
        ),
    ] {
        let (su, errs) = p(src);
        assert!(errs.is_empty(), "{src}: {errs:?}");
        let su = su.unwrap();
        let m = first_module(&su);
        let Some(ModuleItem::Proc(pb)) = m.body.iter().find(|i| matches!(i, ModuleItem::Proc(_)))
        else {
            panic!("no proc block")
        };
        let Stmt::DeferredAssert { region, .. } = &*pb.body else {
            panic!("not DeferredAssert: {:?}", pb.body)
        };
        assert_eq!(*region, want, "{src}");
    }
    // a non-zero `#` delay on an assert is NOT a deferred assert → loud.
    let (_, errs) = p("module m;\ninitial assert #1 (a);\nendmodule");
    assert!(!errs.is_empty(), "`assert #1` must be a loud parse error");
}
