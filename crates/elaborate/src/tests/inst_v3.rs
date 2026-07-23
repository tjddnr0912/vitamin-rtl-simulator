use super::*;

// v3-1: top `tb` instantiating a `dff` submodule → flat SimIr with tb nets +
//       namespaced dff nets + port cont-assigns + 2 Instance records.
#[test]
fn v3_1_top_instantiates_dff_flat_ir() {
    // module dff(input clk, input d, output q); endmodule  (no body logic)
    let dff = module_p(
        "dff",
        vec![],
        vec![
            ansi_port(ast::PortDir::Input, None, "clk"),
            ansi_port(ast::PortDir::Input, None, "d"),
            ansi_port(ast::PortDir::Output, None, "q"),
        ],
        vec![],
    );
    // module tb; reg clk,d,q; dff u_dff(.clk(clk),.d(d),.q(q)); endmodule
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Reg, None, false, &["clk", "d", "q"]),
            inst_named(
                "dff",
                "u_dff",
                vec![
                    ("clk", id_expr("clk")),
                    ("d", id_expr("d")),
                    ("q", id_expr("q")),
                ],
            ),
        ],
    );
    // declaration order: dff then tb → tb is the never-instantiated top.
    let unit = unit_of(vec![dff, tb]);
    let (s, _w) = elab_with_warnings(&unit);

    // 2 instances: tb (parent None) then dff child (parent = 0).
    assert_eq!(s.instances.len(), 2);
    assert!(s.instances[0].parent.is_none());
    assert_eq!(s.instances[0].first_net, 0);
    assert_eq!(s.instances[0].net_count, 3); // tb: clk,d,q
    assert_eq!(s.instances[1].parent, Some(0));
    assert_eq!(s.instances[1].first_net, 3);
    assert_eq!(s.instances[1].net_count, 3); // dff: clk,d,q (namespaced)

    // 6 nets total: tb.clk(0) tb.d(1) tb.q(2) dff.clk(3) dff.d(4) dff.q(5).
    assert_eq!(s.nets.len(), 6);
    // dff ports carry their declared directions.
    assert_eq!(s.nets[3].dir, ir::PortDir::Input); // dff.clk
    assert_eq!(s.nets[4].dir, ir::PortDir::Input); // dff.d
    assert_eq!(s.nets[5].dir, ir::PortDir::Output); // dff.q

    // 3 port cont-assigns. Inputs drive the child port (child = parent_expr);
    // the output drives the parent lvalue (parent = child).
    assert_eq!(s.cont_assigns.len(), 3);
    // walk in child-header order: clk (in), d (in), q (out).
    let ca_clk = &s.cont_assigns[0];
    assert_eq!(ca_clk.lhs.chunks[0].net, 3); // lhs = dff.clk (child input)
    assert!(matches!(
        s.exprs[ca_clk.rhs as usize],
        ir::Expr::Signal { net: 0, .. }
    )); // rhs = tb.clk
    let ca_d = &s.cont_assigns[1];
    assert_eq!(ca_d.lhs.chunks[0].net, 4); // lhs = dff.d
    assert!(matches!(
        s.exprs[ca_d.rhs as usize],
        ir::Expr::Signal { net: 1, .. }
    )); // tb.d
    let ca_q = &s.cont_assigns[2];
    assert_eq!(ca_q.lhs.chunks[0].net, 2); // lhs = tb.q (parent, output port)
    assert!(matches!(
        s.exprs[ca_q.rhs as usize],
        ir::Expr::Signal { net: 5, .. }
    )); // rhs = dff.q
}

// v3-2: top selection — a module never instantiated is the top; the instantiated
//       leaf is NOT a separate top instance at the root.
#[test]
fn v3_2_top_selection_picks_uninstantiated() {
    let leaf = module_p(
        "leaf",
        vec![],
        vec![ansi_port(ast::PortDir::Output, None, "o")],
        vec![],
    );
    let top = module_p(
        "top",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, None, false, &["w"]),
            inst_named("leaf", "u", vec![("o", id_expr("w"))]),
        ],
    );
    let unit = unit_of(vec![leaf, top]);
    let (s, _w) = elab_with_warnings(&unit);
    // root instance is `top` (parent None), and there are exactly 2 instances.
    assert_eq!(s.instances.len(), 2);
    assert!(s.instances[0].parent.is_none());
    assert_eq!(s.instances[0].net_count, 1); // top: w
    assert_eq!(s.instances[1].net_count, 1); // leaf: o
}

// v3-2b: MULTIPLE uninstantiated top modules — IEEE 1364 / iverilog elaborate
//        EVERY uninstantiated module as an independent root (not just the
//        last-declared). Two modules, neither instantiating the other, must BOTH
//        become root instances (parent None) and BOTH bodies be lowered.
#[test]
fn v3_2b_multiple_uninstantiated_tops_all_become_roots() {
    let a = module_p(
        "mod_a",
        vec![],
        vec![],
        vec![proc_item(
            ast::ProcKind::Initial,
            None,
            blk(vec![systask("$display", vec![str_e("a")])]),
        )],
    );
    let b = module_p(
        "mod_b",
        vec![],
        vec![],
        vec![proc_item(
            ast::ProcKind::Initial,
            None,
            blk(vec![systask("$display", vec![str_e("b")])]),
        )],
    );
    let unit = unit_of(vec![a, b]);
    let (s, _w) = elab_with_warnings(&unit);
    // BOTH modules are roots: 2 instances, both parent None.
    assert_eq!(
        s.instances.len(),
        2,
        "both uninstantiated modules must be roots"
    );
    assert!(s.instances[0].parent.is_none());
    assert!(s.instances[1].parent.is_none());
    // BOTH bodies lowered → 2 processes (one initial each).
    assert_eq!(s.processes.len(), 2, "both top bodies must be lowered");
}

// v3-2c: a module instantiated ONLY inside a generate block is NOT a spurious
//        extra root. `holder` instantiates `leaf` inside a `generate` block, so
//        `leaf` is reachable and must appear EXACTLY once (under holder), never as
//        its own root. Guards the multi-root scan against missing generate-nested
//        instantiations (which would double-elaborate `leaf`).
#[test]
fn v3_2c_generate_nested_instance_is_not_a_root() {
    let leaf = module_p(
        "leaf",
        vec![],
        vec![ansi_port(ast::PortDir::Output, None, "o")],
        vec![],
    );
    let holder = module_p(
        "holder",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, None, false, &["w"]),
            ast::ModuleItem::Generate(ast::GenerateConstruct {
                items: vec![ast::GenItem::Block {
                    label: Some(ident("g")),
                    items: vec![ast::GenItem::Item(Box::new(inst_named(
                        "leaf",
                        "u",
                        vec![("o", id_expr("w"))],
                    )))],
                    span: SP,
                }],
                span: SP,
            }),
        ],
    );
    let unit = unit_of(vec![leaf, holder]);
    let (s, _w) = elab_with_warnings(&unit);
    // exactly 2 instances: holder (root) + leaf (under holder via generate).
    assert_eq!(
        s.instances.len(),
        2,
        "leaf instantiated in a generate must not also be a root"
    );
    assert!(s.instances[0].parent.is_none()); // holder
    assert_eq!(s.instances[1].parent, Some(0)); // leaf under holder
}

// v3-3: 2-deep hierarchy tb → mid → leaf. 3 instances, parent chain 0←1←2,
//       contiguous net slices.
#[test]
fn v3_3_two_deep_hierarchy() {
    let leaf = module_p(
        "leaf",
        vec![],
        vec![ansi_port(ast::PortDir::Input, None, "a")],
        vec![],
    );
    let mid = module_p(
        "mid",
        vec![],
        vec![ansi_port(ast::PortDir::Input, None, "x")],
        vec![inst_named("leaf", "u_leaf", vec![("a", id_expr("x"))])],
    );
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Reg, None, false, &["sig"]),
            inst_named("mid", "u_mid", vec![("x", id_expr("sig"))]),
        ],
    );
    let unit = unit_of(vec![leaf, mid, tb]);
    let (s, _w) = elab_with_warnings(&unit);

    assert_eq!(s.instances.len(), 3);
    // depth-first preorder: tb(0), mid(1), leaf(2).
    assert!(s.instances[0].parent.is_none()); // tb
    assert_eq!(s.instances[1].parent, Some(0)); // mid under tb
    assert_eq!(s.instances[2].parent, Some(1)); // leaf under mid
                                                // contiguous slices: tb[0..1] mid[1..2] leaf[2..3].
    assert_eq!(s.instances[0].first_net, 0);
    assert_eq!(s.instances[1].first_net, 1);
    assert_eq!(s.instances[2].first_net, 2);
    assert_eq!(s.nets.len(), 3); // sig, mid.x, leaf.a
                                 // two input-port cont-assigns (mid.x = tb.sig ; leaf.a = mid.x).
    assert_eq!(s.cont_assigns.len(), 2);
    assert_eq!(s.cont_assigns[0].lhs.chunks[0].net, 1); // mid.x
    assert_eq!(s.cont_assigns[1].lhs.chunks[0].net, 2); // leaf.a
}

// v3-4: param override changes a submodule net width. `reg8 #(.W(8))` →
//       child output `[W-1:0] q` is 8 bits.
#[test]
fn v3_4_param_override_changes_width() {
    // module reg8 #(parameter W=1)(output [W-1:0] q); endmodule
    let reg8 = module_p(
        "reg8",
        vec![param("W", 1)],
        vec![ansi_port(ast::PortDir::Output, Some(("W-1", "0")), "q")],
        vec![],
    );
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, Some((7, 0)), false, &["bus"]),
            inst_named_param("reg8", "u", vec![("W", 8)], vec![("q", id_expr("bus"))]),
        ],
    );
    let unit = unit_of(vec![reg8, tb]);
    let (s, _w) = elab_with_warnings(&unit);

    // child q net (index 1: after tb.bus at 0) is width 8 thanks to W=8.
    assert_eq!(s.nets.len(), 2);
    let q = &s.nets[1];
    assert_eq!(q.width, 8, "W override must fold [W-1:0] to width 8");
    assert_eq!(q.msb, 7);
    assert_eq!(q.lsb, 0);
    assert_eq!(q.dir, ir::PortDir::Output);
}

// v3-5: default param (no override) → child width uses the declared default.
#[test]
fn v3_5_param_default_width() {
    let reg_w = module_p(
        "regw",
        vec![param("W", 4)],
        vec![ansi_port(ast::PortDir::Output, Some(("W-1", "0")), "q")],
        vec![],
    );
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, Some((3, 0)), false, &["bus"]),
            inst_named("regw", "u", vec![("q", id_expr("bus"))]),
        ],
    );
    let unit = unit_of(vec![reg_w, tb]);
    let (s, _w) = elab_with_warnings(&unit);
    // child q uses default W=4 → width 4.
    assert_eq!(s.nets[1].width, 4);
}

// v3-6: unconnected input port floats (no cont-assign); unconnected output warns.
#[test]
fn v3_6_unconnected_ports() {
    let leaf = module_p(
        "leaf",
        vec![],
        vec![
            ansi_port(ast::PortDir::Input, None, "a"),
            ansi_port(ast::PortDir::Output, None, "y"),
        ],
        vec![],
    );
    // connect only `a`; leave `y` unconnected.
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Reg, None, false, &["sig"]),
            inst_named("leaf", "u", vec![("a", id_expr("sig"))]),
        ],
    );
    let unit = unit_of(vec![leaf, tb]);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("non-fatal");
    // exactly ONE cont-assign (input a); output y emitted none (floats).
    // net layout: tb.sig=0, leaf.a=1, leaf.y=2.
    assert_eq!(s.cont_assigns.len(), 1);
    assert_eq!(s.cont_assigns[0].lhs.chunks[0].net, 1); // leaf.a (driven input)
                                                        // and exactly one WARNING for the unconnected output port.
    assert_eq!(sink.n_warnings(), 1);
    assert_eq!(sink.n_errors(), 0);
}

// v3-7: unknown module instantiation → ElabUnresolvedInstance error.
#[test]
fn v3_7_unknown_module_errors() {
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, None, false, &["w"]),
            inst_named("nonexistent", "u", vec![("p", id_expr("w"))]),
        ],
    );
    let unit = unit_of(vec![tb]);
    let sink = CollectSink::default();
    let out = elaborate(&unit, &sink);
    assert!(out.is_none(), "unknown module must be a hard error");
    let codes: Vec<_> = sink
        .events
        .borrow()
        .iter()
        .filter_map(|e| match e {
            LogEvent::Diagnostic(d) => Some(d.code),
            _ => None,
        })
        .collect();
    assert!(codes.contains(&MsgCode::ElabUnresolvedInstance));
}

// v3-8: two instances of the SAME leaf under one parent (diamond reuse) is fine,
//       each namespaced separately — NOT flagged as a cycle.
#[test]
fn v3_8_repeated_leaf_instances_not_a_cycle() {
    let leaf = module_p(
        "leaf",
        vec![],
        vec![ansi_port(ast::PortDir::Input, None, "a")],
        vec![],
    );
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Reg, None, false, &["s0", "s1"]),
            inst_named("leaf", "u0", vec![("a", id_expr("s0"))]),
            inst_named("leaf", "u1", vec![("a", id_expr("s1"))]),
        ],
    );
    let unit = unit_of(vec![leaf, tb]);
    let (s, _w) = elab_with_warnings(&unit);
    // tb + 2 leaf instances = 3.
    assert_eq!(s.instances.len(), 3);
    assert_eq!(s.instances[1].parent, Some(0));
    assert_eq!(s.instances[2].parent, Some(0));
    // 2 input cont-assigns: tb.u0.a = tb.s0 ; tb.u1.a = tb.s1.
    assert_eq!(s.cont_assigns.len(), 2);
    assert_eq!(s.cont_assigns[0].lhs.chunks[0].net, 2); // u0.a (s0=0,s1=1,u0.a=2)
    assert_eq!(s.cont_assigns[1].lhs.chunks[0].net, 3); // u1.a
}

// v3-9: determinism — same multi-module AST → byte-identical SimIr.
#[test]
fn v3_9_determinism() {
    let mk = || {
        let dff = module_p(
            "dff",
            vec![],
            vec![
                ansi_port(ast::PortDir::Input, None, "clk"),
                ansi_port(ast::PortDir::Output, None, "q"),
            ],
            vec![],
        );
        let tb = module_p(
            "tb",
            vec![],
            vec![],
            vec![
                netvar(ast::NetVarKind::Reg, None, false, &["clk", "q"]),
                inst_named(
                    "dff",
                    "u",
                    vec![("clk", id_expr("clk")), ("q", id_expr("q"))],
                ),
            ],
        );
        unit_of(vec![dff, tb])
    };
    let (s1, _) = elab_with_warnings(&mk());
    let (s2, _) = elab_with_warnings(&mk());
    assert_eq!(
        s1, s2,
        "same multi-module AST must produce byte-identical SimIr"
    );
}

// v3-10: the v1 single-module path still produces one self-instance covering all
//        nets (top special case: prefix = module name, no children).
#[test]
fn v3_10_single_module_self_instance_preserved() {
    let m = module_p("m", vec![], vec![], vec![wire_vec(7, 0, &["a", "b", "y"])]);
    let unit = unit_of(vec![m]);
    let (s, _w) = elab_with_warnings(&unit);
    assert_eq!(s.instances.len(), 1);
    assert!(s.instances[0].parent.is_none());
    assert_eq!(s.instances[0].first_net, 0);
    assert_eq!(s.instances[0].net_count, 3);
    assert_eq!(s.nets.len(), 3);
}

// v3-11 [Fix 1]: a param override that references the PARENT's param resolves in
//        the parent scope (not folding to 0). top has P=8; child #(.W(P)) → q is 8 bits.
#[test]
fn v3_11_override_uses_parent_param() {
    // module regw #(parameter W=1)(output [W-1:0] q); endmodule
    let regw = module_p(
        "regw",
        vec![param("W", 1)],
        vec![ansi_port(ast::PortDir::Output, Some(("W-1", "0")), "q")],
        vec![],
    );
    // module top #(parameter P=8); wire [P-1:0] bus; regw #(.W(P)) u(.q(bus)); endmodule
    let top = module_p(
        "top",
        vec![param("P", 8)],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, Some((7, 0)), false, &["bus"]),
            // override value is the PARENT param `P` — must resolve to 8 in parent scope.
            inst_named_param_expr(
                "regw",
                "u",
                vec![("W", id_expr("P"))],
                vec![("q", id_expr("bus"))],
            ),
        ],
    );
    let unit = unit_of(vec![regw, top]);
    let (s, _w) = elab_with_warnings(&unit);
    // child q (net 1, after top.bus) must be width 8 — proves P resolved in parent.
    assert_eq!(
        s.nets[1].width, 8,
        "override .W(P) must see parent P=8, not fold to 0"
    );
}

// v3-12 [Fix 1 defensive]: an override of W to 0 → child [W-1:0] underflows →
//        clamped to width 1 + warn, NOT a fatal MAX_NET_WIDTH error.
#[test]
fn v3_12_zero_width_param_does_not_explode() {
    let regw = module_p(
        "regw",
        vec![param("W", 1)],
        vec![ansi_port(ast::PortDir::Output, Some(("W-1", "0")), "q")],
        vec![],
    );
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Wire, None, false, &["bus"]),
            inst_named_param("regw", "u", vec![("W", 0)], vec![("q", id_expr("bus"))]),
        ],
    );
    let unit = unit_of(vec![regw, tb]);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("W=0 must NOT discard the whole IR");
    // child q clamped to width 1, no fatal error.
    assert_eq!(s.nets[1].width, 1);
    assert_eq!(sink.n_errors(), 0);
    assert!(
        sink.n_warnings() >= 1,
        "expected the underflow-clamp warning"
    );
}

// v3-13 [Fix 2]: surplus positional connection → ElabPortMismatch error.
#[test]
fn v3_13_surplus_positional_connection_errors() {
    // dff has 2 ports; connect 3 positionally.
    let dff = module_p(
        "dff",
        vec![],
        vec![
            ansi_port(ast::PortDir::Input, None, "clk"),
            ansi_port(ast::PortDir::Output, None, "q"),
        ],
        vec![],
    );
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Reg, None, false, &["c", "q", "x"]),
            inst_positional(
                "dff",
                "u",
                vec![Some(id_expr("c")), Some(id_expr("q")), Some(id_expr("x"))],
            ),
        ],
    );
    let unit = unit_of(vec![dff, tb]);
    let sink = CollectSink::default();
    let out = elaborate(&unit, &sink);
    assert!(out.is_none(), "surplus connection must be a hard error");
    assert!(diag_codes(&sink).contains(&MsgCode::ElabPortMismatch));
}

// v3-14 [Fix 2]: named connection to a nonexistent port → ElabPortMismatch.
#[test]
fn v3_14_named_ghost_port_errors() {
    let dff = module_p(
        "dff",
        vec![],
        vec![ansi_port(ast::PortDir::Input, None, "clk")],
        vec![],
    );
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Reg, None, false, &["c"]),
            inst_named(
                "dff",
                "u",
                vec![("clk", id_expr("c")), ("ghost", id_expr("c"))],
            ),
        ],
    );
    let unit = unit_of(vec![dff, tb]);
    let sink = CollectSink::default();
    assert!(elaborate(&unit, &sink).is_none());
    assert!(diag_codes(&sink).contains(&MsgCode::ElabPortMismatch));
}

// v3-15 [Fix 3]: an unconnected INOUT warns with "inout", not "output".
#[test]
fn v3_15_unconnected_inout_warning_text() {
    let leaf = module_p(
        "leaf",
        vec![],
        vec![ansi_port(ast::PortDir::Inout, None, "io")],
        vec![],
    );
    let tb = module_p("tb", vec![], vec![], vec![inst_named("leaf", "u", vec![])]);
    let unit = unit_of(vec![leaf, tb]);
    let sink = CollectSink::default();
    elaborate(&unit, &sink).expect("non-fatal");
    let msg = warn_messages(&sink).join("\n");
    assert!(msg.contains("inout port `io`"), "got: {msg}");
    assert!(!msg.contains("output port `io`"));
}

// v3-16 [Fix 1 + Fix 2 happy path]: regression — the clean multi-fix path still
//        produces the exact v3_1 layout (no false positives from the new checks).
#[test]
fn v3_16_happy_path_unaffected_by_new_checks() {
    let dff = module_p(
        "dff",
        vec![],
        vec![
            ansi_port(ast::PortDir::Input, None, "clk"),
            ansi_port(ast::PortDir::Input, None, "d"),
            ansi_port(ast::PortDir::Output, None, "q"),
        ],
        vec![],
    );
    let tb = module_p(
        "tb",
        vec![],
        vec![],
        vec![
            netvar(ast::NetVarKind::Reg, None, false, &["clk", "d", "q"]),
            inst_named(
                "dff",
                "u",
                vec![
                    ("clk", id_expr("clk")),
                    ("d", id_expr("d")),
                    ("q", id_expr("q")),
                ],
            ),
        ],
    );
    let unit = unit_of(vec![dff, tb]);
    let sink = CollectSink::default();
    let s = elaborate(&unit, &sink).expect("clean path");
    assert_eq!(s.instances.len(), 2);
    assert_eq!(s.cont_assigns.len(), 3);
    assert_eq!(sink.n_errors(), 0);
}
