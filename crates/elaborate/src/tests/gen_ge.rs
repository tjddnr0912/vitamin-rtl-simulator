use super::*;

// ge1. generate-for instantiating a `leaf` (input port `a` ← top's `w`) 3× →
//      4 instances (1 top + 3 leaf), top parent None, every leaf parent Some(0).
#[test]
fn ge1_gen_for_instances() {
    let leaf = module_p(
        "leaf",
        vec![],
        vec![ansi_port(ast::PortDir::Input, None, "a")],
        vec![],
    );
    let top = module_p(
        "top",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, None, false, &["w"]),
            generate(vec![gen_for(
                "i",
                dec("0"),
                binop(ast::BinOp::Lt, id_expr("i"), dec("3")),
                binop(ast::BinOp::Add, id_expr("i"), dec("1")),
                Some("g"),
                vec![gitem(inst_named("leaf", "u", vec![("a", id_expr("w"))]))],
            )]),
        ],
    );
    let unit = unit_of(vec![leaf, top]);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("clean generate-for");
    assert_eq!(sink.n_errors(), 0);
    assert_eq!(s.instances.len(), 4); // top + 3 leaf
    assert!(s.instances[0].parent.is_none());
    for inst in &s.instances[1..] {
        assert_eq!(inst.parent, Some(0));
    }
}

// ge2. loop body `wire t; assign t = 1'b0;` ×3 → 3 nets, 3 cont-assigns, the three
//      target nets are DISTINCT (per-iteration g[0].t / g[1].t / g[2].t).
#[test]
fn ge2_gen_for_nets_distinct() {
    let top = module_p(
        "top",
        vec![],
        vec![],
        vec![generate(vec![gen_for(
            "i",
            dec("0"),
            binop(ast::BinOp::Lt, id_expr("i"), dec("3")),
            binop(ast::BinOp::Add, id_expr("i"), dec("1")),
            Some("g"),
            vec![
                gitem(netvar(ast::NetVarKind::Wire, None, false, &["t"])),
                gitem(cont_assign(lv_id("t"), lit("1'b0", ast::IntLitKind::Sized))),
            ],
        )])],
    );
    let unit = unit_of(vec![top]);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("clean generate-for");
    assert_eq!(sink.n_errors(), 0);
    assert_eq!(s.nets.len(), 3);
    assert_eq!(s.cont_assigns.len(), 3);
    let targets: Vec<u32> = s
        .cont_assigns
        .iter()
        .map(|ca| ca.lhs.chunks[0].net)
        .collect();
    let mut uniq = targets.clone();
    uniq.sort_unstable();
    uniq.dedup();
    assert_eq!(
        uniq.len(),
        3,
        "per-iteration nets must not collide: {targets:?}"
    );
}

#[test]
fn ge3_gen_if_true_branch() {
    let unit = build_gen_if(1);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("clean generate-if");
    assert_eq!(sink.n_errors(), 0);
    assert_eq!(s.cont_assigns.len(), 1);
    let rhs = &s.exprs[s.cont_assigns[0].rhs as usize];
    assert!(
        matches!(rhs, ir::Expr::Signal { net: 0, .. }),
        "then = a (net 0)"
    );
}
#[test]
fn ge4_gen_if_false_branch() {
    let unit = build_gen_if(0);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("clean generate-if");
    assert_eq!(sink.n_errors(), 0);
    assert_eq!(s.cont_assigns.len(), 1);
    let rhs = &s.exprs[s.cont_assigns[0].rhs as usize];
    assert!(
        matches!(rhs, ir::Expr::Signal { net: 1, .. }),
        "else = b (net 1)"
    );
}

// ge5. genvar in a net width bound: `wire [i:0] t;` for i in 0..3 → widths [1,2,3].
#[test]
fn ge5_genvar_in_net_width() {
    let top = module_p(
        "top",
        vec![],
        vec![],
        vec![generate(vec![gen_for(
            "i",
            dec("0"),
            binop(ast::BinOp::Lt, id_expr("i"), dec("3")),
            binop(ast::BinOp::Add, id_expr("i"), dec("1")),
            Some("g"),
            vec![gitem(wire_range_expr(id_expr("i"), &["t"]))],
        )])],
    );
    let unit = unit_of(vec![top]);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("clean generate-for");
    assert_eq!(sink.n_errors(), 0);
    let widths: Vec<u32> = s.nets.iter().map(|n| n.width).collect();
    assert_eq!(widths, vec![1, 2, 3]);
}

// ge6. determinism: elaborate the same unit twice → byte-identical arenas.
#[test]
fn ge6_gen_determinism() {
    let mk = || {
        let top = module_p(
            "top",
            vec![],
            vec![],
            vec![generate(vec![gen_for(
                "i",
                dec("0"),
                binop(ast::BinOp::Lt, id_expr("i"), dec("4")),
                binop(ast::BinOp::Add, id_expr("i"), dec("1")),
                Some("g"),
                vec![
                    gitem(netvar(ast::NetVarKind::Wire, None, false, &["t"])),
                    gitem(cont_assign(lv_id("t"), lit("1'b0", ast::IntLitKind::Sized))),
                ],
            )])],
        );
        unit_of(vec![top])
    };
    let a = elaborate(&mk(), &CollectSink::default()).expect("clean");
    let b = elaborate(&mk(), &CollectSink::default()).expect("clean");
    assert_eq!(a.nets, b.nets);
    assert_eq!(a.instances, b.instances);
    assert_eq!(a.cont_assigns, b.cont_assigns);
}

// ge7. (M1 guard) a stuck genvar step `i = i` → exactly ONE ElabUnsupported, NOT
//      ~4096 duplicate-decl errors.
#[test]
fn ge7_stuck_genvar_one_error() {
    let top = module_p(
        "top",
        vec![],
        vec![],
        vec![generate(vec![gen_for(
            "i",
            dec("0"),
            binop(ast::BinOp::Lt, id_expr("i"), dec("5")),
            id_expr("i"), // step = i (no advance) → stall
            Some("g"),
            vec![gitem(netvar(ast::NetVarKind::Wire, None, false, &["t"]))],
        )])],
    );
    let unit = unit_of(vec![top]);
    let sink = CollectSink::default();
    let _ = elaborate(&unit, &sink); // returns None (had_error); must not flood
    assert_eq!(
        sink.n_errors(),
        1,
        "stuck genvar must emit exactly one error"
    );
    assert!(err_codes(&sink).contains(&MsgCode::ElabUnsupported));
}
