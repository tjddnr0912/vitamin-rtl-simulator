use super::*;

// v2-1: initial testbench — $dumpfile/$dumpvars + a=0 + #5 + a=1 + #5 + $display + $finish.
#[test]
fn v2_1_initial_testbench_structure() {
    let body = blk(vec![
        systask("$dumpfile", vec![str_e("dump.vcd")]),
        systask("$dumpvars", vec![dec("0"), id_expr("a")]),
        bassign("a", dec("0")),
        delay_stmt(5, None),
        bassign("a", dec("1")),
        delay_stmt(5, None),
        systask("$display", vec![str_e("a=%d"), id_expr("a")]),
        systask("$finish", vec![]),
    ]);
    let unit = module(
        "tb",
        vec![
            netvar(ast::NetVarKind::Reg, Some((0, 0)), false, &["a"]),
            proc_item(ast::ProcKind::Initial, None, body),
        ],
    );
    let (ir, warns) = elab_with_warnings(&unit);
    assert_eq!(warns, 0, "clean testbench must not warn");
    assert_eq!(ir.processes.len(), 1);
    let p = &ir.processes[0];
    assert_eq!(p.sensitivity.kind, ir::SensKind::Initial);
    assert!(p.sensitivity.edges.is_empty());
    assert_cfg_valid(p);
    assert_all_paths_return(p);
    // two #5 delays → two Delay terminators with Active region. Since
    // format_version 4 `amount` is an ExprId — resolve each to its folded
    // Const value (raw module units; the engine scales at suspension time).
    let delays: Vec<_> = p
        .body
        .iter()
        .filter_map(|bb| match bb.term {
            ir::Terminator::Delay { amount, region, .. } => {
                let v = match &ir.exprs[amount as usize] {
                    ir::Expr::Const { val } => ir.consts[*val as usize]
                        .bits
                        .val
                        .first()
                        .copied()
                        .unwrap_or(u64::MAX),
                    other => panic!("const #5 must lower to a Const expr, got {other:?}"),
                };
                Some((v, region))
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        delays,
        vec![(5, ir::DelayRegion::Active), (5, ir::DelayRegion::Active)]
    );
}

// v2-2: always_ff @(posedge clk) q <= d → SensKind::Edge / Posedge.
#[test]
fn v2_2_always_ff_edge() {
    let body = nb("q", id_expr("d"));
    let unit = module(
        "ff",
        vec![
            netvar(
                ast::NetVarKind::Reg,
                Some((0, 0)),
                false,
                &["q", "clk", "d"],
            ),
            proc_item(
                ast::ProcKind::AlwaysFf,
                Some(ev_list(vec![(ast::Edge::Posedge, "clk")])),
                body,
            ),
        ],
    );
    let (ir, _) = elab_with_warnings(&unit);
    let p = &ir.processes[0];
    assert_eq!(p.sensitivity.kind, ir::SensKind::Edge);
    assert_eq!(p.sensitivity.edges.len(), 1);
    assert_eq!(p.sensitivity.edges[0].kind, ir::EdgeKind::Posedge);
    assert_cfg_valid(p);
    assert_all_paths_return(p);
}

// v2-3: bare always @(a or b) → SensKind::Level, both AnyEdge terms.
#[test]
fn v2_3_level_sensitivity() {
    let body = bassign("y", id_expr("a"));
    let unit = module(
        "lvl",
        vec![
            netvar(ast::NetVarKind::Reg, Some((0, 0)), false, &["y", "a", "b"]),
            proc_item(
                ast::ProcKind::Always,
                Some(ev_list(vec![
                    (ast::Edge::NoEdge, "a"),
                    (ast::Edge::NoEdge, "b"),
                ])),
                body,
            ),
        ],
    );
    let (ir, _) = elab_with_warnings(&unit);
    let p = &ir.processes[0];
    assert_eq!(p.sensitivity.kind, ir::SensKind::Level);
    assert_eq!(p.sensitivity.edges.len(), 2);
    assert_cfg_valid(p);
    assert_all_paths_return(p);
}

// v2-4 (M-C): bare `always #5 clk = ~clk;` clock generator → NON-FATAL, Comb,
// forever-wrapped (no Return-reachable continuation; back-edge cycle).
#[test]
fn v2_4_clock_generator_self_timed() {
    let invert = ex(ast::ExprKind::Unary {
        op: ast::UnOp::BitNot,
        operand: Box::new(id_expr("clk")),
    });
    let body = delay_stmt(5, Some(bassign("clk", invert)));
    let unit = module(
        "clkgen",
        vec![
            netvar(ast::NetVarKind::Reg, Some((0, 0)), false, &["clk"]),
            proc_item(ast::ProcKind::Always, None, body), // <-- no header @, in-body #5
        ],
    );
    let (ir, warns) = elab_with_warnings(&unit);
    assert_eq!(
        warns, 0,
        "a self-timed clock generator is legal, must not warn"
    );
    let p = &ir.processes[0];
    assert_eq!(p.sensitivity.kind, ir::SensKind::Comb);
    assert_cfg_valid(p); // forever is exempt from assert_all_paths_return
                         // there is a Delay terminator and a back-edge Goto (the forever cycle).
    assert!(p
        .body
        .iter()
        .any(|bb| matches!(bb.term, ir::Terminator::Delay { .. })));
}

// v2-5 (M-C): truly inert `always` (no @ no timing) → WARN, still Some + valid.
#[test]
fn v2_5_bare_always_no_timing_warns_not_fatal() {
    let unit = module(
        "m",
        vec![
            netvar(ast::NetVarKind::Reg, Some((0, 0)), false, &["a"]),
            proc_item(ast::ProcKind::Always, None, bassign("a", dec("0"))),
        ],
    );
    let sink = CollectSink::default();
    let out = elaborate(&unit, &sink);
    assert!(out.is_some(), "bare always is now non-fatal");
    assert_eq!(sink.n_warnings(), 1);
    assert_cfg_valid(&out.unwrap().processes[0]);
}

// v2-6: if/else → Branch + shared merge; every path Returns.
#[test]
fn v2_6_if_else_merge() {
    let body = ast::Stmt::If {
        cond: id_expr("c"),
        then_s: Box::new(bassign("y", dec("1"))),
        else_s: Some(Box::new(bassign("y", dec("0")))),
        span: SP,
    };
    let unit = module(
        "m",
        vec![
            netvar(ast::NetVarKind::Reg, Some((0, 0)), false, &["y", "c"]),
            proc_item(ast::ProcKind::Initial, None, body),
        ],
    );
    let (ir, _) = elab_with_warnings(&unit);
    let p = &ir.processes[0];
    assert!(p
        .body
        .iter()
        .any(|bb| matches!(bb.term, ir::Terminator::Branch { .. })));
    assert_cfg_valid(p);
    assert_all_paths_return(p);
}

// v2-7 (M-B): casez lowers (NON-FATAL, warning) into a CaseEq Branch chain.
#[test]
fn v2_7_casez_wildcard_free_lowers_cleanly() {
    let items = vec![
        ast::CaseItem::Match {
            labels: vec![lit("2'b10", ast::IntLitKind::Sized)],
            body: Box::new(bassign("y", dec("1"))),
            span: SP,
        },
        ast::CaseItem::Default {
            body: Box::new(bassign("y", dec("0"))),
            span: SP,
        },
    ];
    let body = ast::Stmt::Case {
        kind: ast::CaseKind::Casez,
        scrutinee: id_expr("s"),
        items,
        span: SP,
    };
    let unit = module(
        "m",
        vec![
            netvar(ast::NetVarKind::Reg, Some((1, 0)), false, &["s", "y"]),
            proc_item(ast::ProcKind::Initial, None, body),
        ],
    );
    let (ir, warns) = elab_with_warnings(&unit);
    // v7: casez/casex lower to ONE dedicated match op per label (z-only /
    // x-or-z don't-care on EITHER side, computed at runtime), with NO warning.
    assert_eq!(warns, 0, "casez label lowers cleanly, no warning");
    let p = &ir.processes[0];
    let has_casez_eq = ir.exprs.iter().any(|e| {
        matches!(
            e,
            ir::Expr::Binary {
                op: ir::BinOp::CasezEq,
                ..
            }
        )
    });
    assert!(has_casez_eq, "casez must lower via BinOp::CasezEq (v7)");
    assert!(p
        .body
        .iter()
        .any(|bb| matches!(bb.term, ir::Terminator::Branch { .. })));
    assert_cfg_valid(p);
    assert_all_paths_return(p);
}

// v2-8: in-body @(posedge clk) → Wait{Edge,Posedge}, NOT process sensitivity.
#[test]
fn v2_8_in_body_event_wait() {
    let body = blk(vec![
        ast::Stmt::EventCtrl {
            ctrl: ev_list(vec![(ast::Edge::Posedge, "clk")]),
            body: None,
            span: SP,
        },
        nb("q", id_expr("d")),
    ]);
    let unit = module(
        "m",
        vec![
            netvar(
                ast::NetVarKind::Reg,
                Some((0, 0)),
                false,
                &["q", "d", "clk"],
            ),
            proc_item(ast::ProcKind::Initial, None, body),
        ],
    );
    let (ir, _) = elab_with_warnings(&unit);
    let p = &ir.processes[0];
    assert_eq!(p.sensitivity.kind, ir::SensKind::Initial); // block-level stays Initial
    let waits: Vec<_> = p
        .body
        .iter()
        .filter_map(|bb| match &bb.term {
            ir::Terminator::Wait {
                cond: ir::WaitCause::Edge { kind, .. },
                ..
            } => Some(*kind),
            _ => None,
        })
        .collect();
    assert_eq!(waits, vec![ir::EdgeKind::Posedge]);
    assert_cfg_valid(p);
    assert_all_paths_return(p);
}

// v2-9 (M-D): unknown $task ($printtimescale — still unmapped) → WARN + skip,
// IR survives, no Stmt. ($timeformat, the old example here, is now SUPPORTED —
// §21.3.2 — so it emits a real no-op Display stmt instead of warn-skipping.)
#[test]
fn v2_9_unknown_systask_nonfatal() {
    let body = blk(vec![
        systask("$printtimescale", vec![]),
        bassign("a", dec("0")),
        systask("$finish", vec![]),
    ]);
    let unit = module(
        "tb",
        vec![
            netvar(ast::NetVarKind::Reg, Some((0, 0)), false, &["a"]),
            proc_item(ast::ProcKind::Initial, None, body),
        ],
    );
    let (ir, warns) = elab_with_warnings(&unit);
    assert_eq!(warns, 1, "$printtimescale must warn-skip, not kill the IR");
    // exactly one SysTask stmt survives ($finish); $printtimescale emitted nothing.
    let n_systask = ir
        .stmts
        .iter()
        .filter(|s| matches!(s, ir::Stmt::SysTask { .. }))
        .count();
    assert_eq!(n_systask, 1);
    assert_cfg_valid(&ir.processes[0]);
    assert_all_paths_return(&ir.processes[0]);
}

// v2-10: full multi-process testbench (initial stimulus + always_ff DUT) +
//        whole-SimIr determinism (same AST → byte-identical SimIr).
#[test]
fn v2_10_multiprocess_and_determinism() {
    let mk = || {
        let dut = nb("q", id_expr("d"));
        let stim = blk(vec![
            bassign("d", dec("1")),
            delay_stmt(10, None),
            systask("$finish", vec![]),
        ]);
        module(
            "tb",
            vec![
                netvar(
                    ast::NetVarKind::Reg,
                    Some((0, 0)),
                    false,
                    &["q", "d", "clk"],
                ),
                proc_item(
                    ast::ProcKind::AlwaysFf,
                    Some(ev_list(vec![(ast::Edge::Posedge, "clk")])),
                    dut,
                ),
                proc_item(ast::ProcKind::Initial, None, stim),
            ],
        )
    };
    let (ir1, _) = elab_with_warnings(&mk());
    let (ir2, _) = elab_with_warnings(&mk());
    assert_eq!(ir1, ir2, "same AST must produce byte-identical SimIr");
    assert_eq!(ir1.processes.len(), 2);
    assert_eq!(ir1.processes[0].sensitivity.kind, ir::SensKind::Edge); // DUT
    assert_eq!(ir1.processes[1].sensitivity.kind, ir::SensKind::Initial); // stimulus
    for p in &ir1.processes {
        assert_cfg_valid(p);
    }
    assert_all_paths_return(&ir1.processes[1]); // initial terminates
}

// ════════════════════════════════════════════════════════════════════
//  v3 — module instantiation + hierarchy flattening tests
// ════════════════════════════════════════════════════════════════════

impl CollectSink {
    /// Count ERROR-severity diagnostics.
    pub(crate) fn n_errors(&self) -> usize {
        self.events
            .borrow()
            .iter()
            .filter(|e| matches!(e, LogEvent::Diagnostic(d) if d.severity == diag::Severity::Error))
            .count()
    }
}
