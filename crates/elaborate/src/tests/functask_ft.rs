use super::*;

// ft-e1. combinational function `function [7:0] add1(input [7:0] x); add1=x+1;
//        endfunction` called as `assign y = add1(a);` → the Call inlines to
//        Binary(Add, Signal a, Const 1) (under a return-width resize) and y's
//        cont-assign points at it.
#[test]
fn ft_e1_function_inlines_to_return_expr() {
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a", "y"]),
            func_def(
                "add1",
                Some((7, 0)),
                vec![tf_port(ast::PortDir::Input, Some((7, 0)), "x")],
                vec![],
                bassign("add1", binop(ast::BinOp::Add, id_expr("x"), dec("1"))),
            ),
            cont_assign(lv_id("y"), call("add1", vec![id_expr("a")])),
        ],
    );
    let s = elab_ok(&unit);
    // funcs arena stays empty (inline path, no call-frame schema).
    assert!(s.funcs.is_empty());
    assert!(s.blocks.is_empty());
    // one cont-assign onto y (net 1); rhs = Binary(Add, Signal a (net 0), Const 1)
    // under the §10.7 return-width resize (add1 is 8-bit; `x+1` is a 32-bit add
    // via the unsized `1`, truncated to the 8-bit return).
    assert_eq!(s.cont_assigns.len(), 1);
    let ca = &s.cont_assigns[0];
    assert_eq!(ca.lhs.chunks[0].net, 1); // y is 2nd net
    match &s.exprs[peel_resize(&s, ca.rhs) as usize] {
        ir::Expr::Binary {
            op: ir::BinOp::Add,
            lhs,
            rhs,
        } => {
            // `peel_resize` strips the `$unsigned` the formal bind stamps onto the
            // actual (§4.5.325: the bind IS an assignment to the formal's declared
            // type, and the stamp is what seals it) — the operand under it is `a`.
            assert!(matches!(
                s.exprs[peel_resize(&s, *lhs) as usize],
                ir::Expr::Signal { net: 0, word: None } // the actual arg `a`
            ));
            match &s.exprs[*rhs as usize] {
                ir::Expr::Const { val } => assert_eq!(s.consts[*val as usize].bits.val[0], 1),
                other => panic!("expected Const 1, got {other:?}"),
            }
        }
        other => panic!("expected Binary(Add, …), got {other:?}"),
    }
}

// ft-e2. straight-line function with a LOCAL var (SSA-by-substitution):
//        function [7:0] f(input [7:0] x); reg [7:0] t; begin t=x+1; f=t+1; end
//        → assign y=f(a)  ==  ((a+1)+1). The local `t` becomes NO net (no extra
//        nets beyond a,y); it is folded into the substitution scope.
#[test]
fn ft_e2_function_local_var_folds() {
    let body = blk(vec![
        bassign("t", binop(ast::BinOp::Add, id_expr("x"), dec("1"))),
        bassign("f", binop(ast::BinOp::Add, id_expr("t"), dec("1"))),
    ]);
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a", "y"]),
            func_def(
                "f",
                Some((7, 0)),
                vec![tf_port(ast::PortDir::Input, Some((7, 0)), "x")],
                vec![netvar_decl_reg("t")],
                body,
            ),
            cont_assign(lv_id("y"), call("f", vec![id_expr("a")])),
        ],
    );
    let s = elab_ok(&unit);
    // exactly 2 nets (a, y) — the local `t` created NONE.
    assert_eq!(s.nets.len(), 2);
    // rhs root = Add( Add(Signal a, 1), 1 ), each Add under a return/local-width
    // resize (8-bit return + 8-bit local `t`, both fed by 32-bit `x+1` adds).
    let root = &s.exprs[peel_resize(&s, s.cont_assigns[0].rhs) as usize];
    let ir::Expr::Binary {
        op: ir::BinOp::Add,
        lhs,
        rhs,
    } = root
    else {
        panic!("expected outer Add, got {root:?}");
    };
    // outer rhs is Const 1
    assert!(matches!(&s.exprs[*rhs as usize], ir::Expr::Const { .. }));
    // outer lhs is Add(Signal a, Const 1) (under the local-`t` resize)
    match &s.exprs[peel_resize(&s, *lhs) as usize] {
        ir::Expr::Binary {
            op: ir::BinOp::Add,
            lhs: l2,
            ..
        } => assert!(matches!(
            // under the `$unsigned` the formal bind stamps on the actual (§4.5.325)
            s.exprs[peel_resize(&s, *l2) as usize],
            ir::Expr::Signal { net: 0, .. }
        )),
        other => panic!("expected inner Add, got {other:?}"),
    }
}

// ft-e3. nested non-recursive function call (f calls g) folds fully.
//        function g(input x); g = x + 1; endfunction
//        function f(input x); f = g(x) + 1; endfunction  → y=f(a) == ((a+1)+1)
#[test]
fn ft_e3_nested_function_calls() {
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a", "y"]),
            func_def(
                "g",
                Some((7, 0)),
                vec![tf_port(ast::PortDir::Input, Some((7, 0)), "x")],
                vec![],
                bassign("g", binop(ast::BinOp::Add, id_expr("x"), dec("1"))),
            ),
            func_def(
                "f",
                Some((7, 0)),
                vec![tf_port(ast::PortDir::Input, Some((7, 0)), "x")],
                vec![],
                bassign(
                    "f",
                    binop(ast::BinOp::Add, call("g", vec![id_expr("x")]), dec("1")),
                ),
            ),
            cont_assign(lv_id("y"), call("f", vec![id_expr("a")])),
        ],
    );
    let s = elab_ok(&unit);
    // outer is f = g(a) + 1, under f's 8-bit return resize.
    let ir::Expr::Binary {
        op: ir::BinOp::Add,
        lhs,
        ..
    } = &s.exprs[peel_resize(&s, s.cont_assigns[0].rhs) as usize]
    else {
        panic!("expected outer Add");
    };
    // inner is g(a) = Add(Signal a, 1), under g's own 8-bit return resize.
    match &s.exprs[peel_resize(&s, *lhs) as usize] {
        ir::Expr::Binary {
            op: ir::BinOp::Add,
            lhs: l2,
            ..
        } => assert!(matches!(
            // under the `$unsigned` the formal bind stamps on the actual (§4.5.325)
            s.exprs[peel_resize(&s, *l2) as usize],
            ir::Expr::Signal { net: 0, .. }
        )),
        other => panic!("expected inner g() Add, got {other:?}"),
    }
}

// ft-e4. simple task writing an OUTPUT formal, called in an initial block:
//        task setq(input [7:0] d, output [7:0] q); q = d; endtask
//        initial setq(a, y);  → the body's `q = d` lowers to BlockingAssign onto
//        the caller's net y, rhs = the caller's net a.
#[test]
fn ft_e4_task_output_writeback_inline() {
    let unit = module(
        "m",
        vec![
            netvar(ast::NetVarKind::Reg, Some((7, 0)), false, &["a", "y"]),
            task_def(
                "setq",
                vec![
                    tf_port(ast::PortDir::Input, Some((7, 0)), "d"),
                    tf_port(ast::PortDir::Output, Some((7, 0)), "q"),
                ],
                vec![],
                bassign("q", id_expr("d")),
            ),
            proc_item(
                ast::ProcKind::Initial,
                None,
                blk(vec![task_call("setq", vec![id_expr("a"), id_expr("y")])]),
            ),
        ],
    );
    let s = elab_ok(&unit);
    // §13.5.1/§13.5.3 copy-in/copy-out. Nets: a (=0), y (=1), then the formal-width
    // locals — the INPUT d (=2) and the OUTPUT q (=3). The entry block holds THREE
    // statements: copy-in `d_local = a`, the body `q_local = d_local`, and the
    // copy-out `y = q_local`.
    assert_eq!(s.processes.len(), 1);
    let p = &s.processes[0];
    let entry = &p.body[p.entry as usize];
    assert_eq!(entry.stmts.len(), 3);
    let assign = |i: usize| -> (u32, &ir::Expr) {
        match &s.stmts[entry.stmts[i] as usize] {
            ir::Stmt::BlockingAssign { lhs, rhs } => (lhs.chunks[0].net, &s.exprs[*rhs as usize]),
            other => panic!("expected BlockingAssign at stmt {i}, got {other:?}"),
        }
    };
    // stmt[0]: copy-IN d_local (=2) = a (=0).
    assert!(matches!(assign(0), (2, ir::Expr::Signal { net: 0, .. })));
    // stmt[1]: body q_local (=3) = d_local (=2).
    assert!(matches!(assign(1), (3, ir::Expr::Signal { net: 2, .. })));
    // stmt[2]: copy-OUT y (=1) = q_local (=3).
    assert!(matches!(assign(2), (1, ir::Expr::Signal { net: 3, .. })));
}

// ft-e5. unknown function call → E-ELAB-UNRESOLVED-NAME (IR discarded).
#[test]
fn ft_e5_unknown_function_errors() {
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a", "y"]),
            cont_assign(lv_id("y"), call("nope", vec![id_expr("a")])),
        ],
    );
    let sink = CollectSink::default();
    let out = elaborate(&unit, &sink);
    assert!(out.is_none(), "unknown function must fail elaboration");
    assert!(err_codes(&sink).contains(&MsgCode::ElabUnresolvedName));
}

// ft-e6. recursive function → B1 frame-call: lowered to the func arena (no
//        infinite inline expansion), the call site emits an `Expr::Call`.
#[test]
fn ft_e6_recursive_function_framed() {
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a", "y"]),
            func_def(
                "rec",
                Some((7, 0)),
                vec![tf_port(ast::PortDir::Input, Some((7, 0)), "x")],
                vec![],
                // rec = rec(x) + 1  → self-call inside its own body
                bassign(
                    "rec",
                    binop(ast::BinOp::Add, call("rec", vec![id_expr("x")]), dec("1")),
                ),
            ),
            cont_assign(lv_id("y"), call("rec", vec![id_expr("a")])),
        ],
    );
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("recursive function now frames (B1)");
    // exactly one FuncDef, well-formed (entry in range, 1 param, not a task).
    assert_eq!(s.funcs.len(), 1, "one frame func");
    let fd = s.funcs[0];
    assert!(!fd.is_task);
    assert_eq!(fd.n_params, 1);
    assert!(
        (fd.entry as usize) < s.blocks.len(),
        "entry indexes the func arena"
    );
    // the cont-assign RHS is an Expr::Call to func 0 with one arg.
    let ca = &s.cont_assigns[0];
    match &s.exprs[ca.rhs as usize] {
        ir::Expr::Call { func, args } => {
            assert_eq!(*func, 0);
            assert_eq!(args.len(), 1);
        }
        other => panic!("cont-assign RHS must be a frame Call, got {other:?}"),
    }
    // the recursive call inside the body lowered to a Call too (no inline blowup).
    assert!(
        s.exprs
            .iter()
            .filter(|e| matches!(e, ir::Expr::Call { func: 0, .. }))
            .count()
            >= 2,
        "both the call site AND the body self-call are Expr::Call"
    );
}

// ft-e7. function whose body has CONTROL FLOW (if/else) → B1 frame-call: lowered
//        to a multi-BB CFG (the `if` is a `Branch` terminator), no longer rejected.
#[test]
fn ft_e7_control_flow_function_framed() {
    let if_body = ast::Stmt::If {
        cond: id_expr("x"),
        then_s: Box::new(bassign("f", dec("1"))),
        else_s: Some(Box::new(bassign("f", dec("0")))),
        span: SP,
    };
    let unit = module(
        "m",
        vec![
            wire_vec(7, 0, &["a", "y"]),
            func_def(
                "f",
                Some((7, 0)),
                vec![tf_port(ast::PortDir::Input, Some((7, 0)), "x")],
                vec![],
                if_body,
            ),
            cont_assign(lv_id("y"), call("f", vec![id_expr("a")])),
        ],
    );
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("control-flow function now frames (B1)");
    assert_eq!(s.funcs.len(), 1);
    let fd = s.funcs[0];
    assert!((fd.entry as usize) < s.blocks.len());
    // the `if/else` lowered to >1 block, and the entry block branches (multi-BB
    // exercises the +base terminator rebase — a single-BB body would not).
    assert!(s.blocks.len() >= 2, "if/else is a multi-BB CFG");
    assert!(
        matches!(
            s.blocks[fd.entry as usize].term,
            ir::Terminator::Branch { .. }
        ),
        "the func entry block ends in a Branch (the `if`)"
    );
    // every Branch/Goto target in the func arena is in range (rebase correct).
    for b in &s.blocks {
        match b.term {
            ir::Terminator::Branch {
                then_bb, else_bb, ..
            } => {
                assert!((then_bb as usize) < s.blocks.len());
                assert!((else_bb as usize) < s.blocks.len());
            }
            ir::Terminator::Goto { target } => assert!((target as usize) < s.blocks.len()),
            _ => {}
        }
    }
    // the call site is an Expr::Call.
    let ca = &s.cont_assigns[0];
    assert!(matches!(
        &s.exprs[ca.rhs as usize],
        ir::Expr::Call { func: 0, .. }
    ));
}

#[test]
fn nested_fork_is_unsupported_error() {
    // initial fork begin fork a=1; join end join
    // The INNER fork (inside the OUTER fork's child) is the nested case → error.
    let inner = fork_stmt(vec![bassign("a", dec("1"))], ast::JoinKind::Join);
    let child = blk(vec![inner]);
    let outer = fork_stmt(vec![child], ast::JoinKind::Join);
    let unit = module(
        "m",
        vec![
            netvar(ast::NetVarKind::Reg, None, false, &["a"]),
            proc_item(ast::ProcKind::Initial, None, outer),
        ],
    );
    let sink = CollectSink::default();
    let (ir, modes) = elaborate_with_modes(&unit, &sink);
    // Inner fork (inside a fork child) → ElabUnsupported error. Elaborate still
    // produces no SimIr (had_error set), but the OUTER fork's mode WAS recorded and
    // the inner fork recorded NO mode entry.
    assert!(diag_codes(&sink).contains(&MsgCode::ElabUnsupported));
    // Only the OUTER fork's mode is recorded; the inner one is rejected.
    assert_eq!(modes.len(), 1);
    assert!(ir.is_none(), "design is rejected by the nested-fork error");
}
