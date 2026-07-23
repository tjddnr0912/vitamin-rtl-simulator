use super::*;

// I1. named connections: dff u1(.clk(clk), .d(d), .q(q));
#[test]
fn i1_named_connections() {
    let mi = inst_of("dff u1(.clk(clk), .d(d), .q(q));");
    assert_eq!(mi.module_name.name, "dff");
    assert!(mi.param_overrides.is_empty());
    assert_eq!(mi.instances.len(), 1);
    let it = &mi.instances[0];
    assert_eq!(it.name.name, "u1");
    let PortConnList::Named(conns, _) = &it.conns else {
        panic!("not named")
    };
    assert_eq!(conns.len(), 3);
    assert_eq!(conns[0].name.name, "clk");
    assert_eq!(id_name(conns[0].value.as_ref().unwrap()), "clk");
    assert_eq!(conns[2].name.name, "q");
    assert_eq!(id_name(conns[2].value.as_ref().unwrap()), "q");
}

// I2. positional connections: dff u1(clk, d, q);
#[test]
fn i2_positional_connections() {
    let mi = inst_of("dff u1(clk, d, q);");
    assert_eq!(mi.module_name.name, "dff");
    let PortConnList::Positional(conns) = &mi.instances[0].conns else {
        panic!("not positional")
    };
    assert_eq!(conns.len(), 3);
    assert_eq!(id_name(conns[0].as_ref().unwrap()), "clk");
    assert_eq!(id_name(conns[1].as_ref().unwrap()), "d");
    assert_eq!(id_name(conns[2].as_ref().unwrap()), "q");
}

// I3. named param override: reg8 #(.W(8)) r(.d(d), .q(q));
#[test]
fn i3_named_param_override() {
    let mi = inst_of("reg8 #(.W(8)) r(.d(d), .q(q));");
    assert_eq!(mi.module_name.name, "reg8");
    assert_eq!(mi.param_overrides.len(), 1);
    let ParamConn::Named { name, value, .. } = &mi.param_overrides[0] else {
        panic!("not a named override")
    };
    assert_eq!(name.name, "W");
    assert!(matches!(
        value.as_ref().unwrap().kind,
        ExprKind::IntLit { .. }
    ));
    assert_eq!(mi.instances[0].name.name, "r");
}

// I4. positional param override + multiple params: mem #(8, 256) u(.clk(clk));
#[test]
fn i4_positional_param_override() {
    let mi = inst_of("mem #(8, 256) u(.clk(clk));");
    assert_eq!(mi.param_overrides.len(), 2);
    assert!(matches!(mi.param_overrides[0], ParamConn::Positional(_)));
    assert!(matches!(mi.param_overrides[1], ParamConn::Positional(_)));
}

// I5. multiple instances per statement: dff u0(clk,q0), u1(q0,q1);
#[test]
fn i5_multiple_instances_per_statement() {
    let mi = inst_of("dff u0(clk, q0), u1(q0, q1);");
    assert_eq!(mi.module_name.name, "dff");
    assert_eq!(mi.instances.len(), 2);
    assert_eq!(mi.instances[0].name.name, "u0");
    assert_eq!(mi.instances[1].name.name, "u1");
}

// I6. unconnected positional slot: alu u(a, , c);  → None in the middle.
#[test]
fn i6_positional_unconnected_slot() {
    let mi = inst_of("alu u(a, , c);");
    let PortConnList::Positional(conns) = &mi.instances[0].conns else {
        panic!("not positional")
    };
    assert_eq!(conns.len(), 3);
    assert!(conns[0].is_some());
    assert!(conns[1].is_none()); // skipped port
    assert!(conns[2].is_some());
}

// I7. explicitly-unconnected named port `.q()` ⇒ value None; empty `()` list.
#[test]
fn i7_named_empty_and_empty_list() {
    let mi = inst_of("dff u1(.clk(clk), .q());");
    let PortConnList::Named(conns, _) = &mi.instances[0].conns else {
        panic!("not named")
    };
    assert_eq!(conns.len(), 2);
    assert!(conns[1].value.is_none(), "`.q()` ⇒ None");
    // empty `()` list ⇒ zero-arity Positional
    let mi2 = inst_of("noports u2();");
    let PortConnList::Positional(c2) = &mi2.instances[0].conns else {
        panic!("empty () should be Positional")
    };
    assert!(c2.is_empty());
}

// I8. instance-array dim + a connection expr: rep u_x [3:0] (.in(bus));
#[test]
fn i8_instance_array_dim() {
    let mi = inst_of("rep u_x [3:0] (.in(bus));");
    let it = &mi.instances[0];
    assert_eq!(it.name.name, "u_x");
    assert_eq!(it.unpacked.len(), 1);
    assert!(matches!(it.unpacked[0], Dim::Range(_)));
}

// I9. expression-valued named connection: dff u(.d(a & b), .q(q));
#[test]
fn i9_expression_connection() {
    let mi = inst_of("dff u(.d(a & b), .q(q));");
    let PortConnList::Named(conns, _) = &mi.instances[0].conns else {
        panic!("not named")
    };
    let (op, _l, _r) = bin(conns[0].value.as_ref().unwrap());
    assert_eq!(op, BinOp::BitAnd);
}

// I10. `.*` implicit wildcard connection now parses cleanly (it used to be a
//      stub-with-advisory; the wildcard is now supported, IEEE §23.3.2.5), and
//      the trailing item still parses.
#[test]
fn i10_dotstar_parses_as_wildcard() {
    let (su, errs) = p("module m; sub u1(.*); assign y = a;\nendmodule");
    assert!(errs.is_empty(), "`.*` should no longer emit an advisory");
    let su = su.unwrap();
    let m = first_module(&su);
    // the instance is present with an empty explicit list + wildcard = true.
    let inst = m
        .body
        .iter()
        .find_map(|i| match i {
            ModuleItem::Instance(it) => Some(it),
            _ => None,
        })
        .expect("instance present");
    let PortConnList::Named(conns, wildcard) = &inst.instances[0].conns else {
        panic!("expected a Named conn list for `.*`");
    };
    assert!(conns.is_empty(), "`.*` alone has no explicit conns");
    assert!(*wildcard, "`.*` sets the wildcard flag");
    // …and the trailing assign still parses.
    assert!(m
        .body
        .iter()
        .any(|i| matches!(i, ModuleItem::ContAssign(_))));
}

// g1. genvar multi-declaration → Genvar{names==["i","j"]}.
#[test]
fn g1_genvar_decl() {
    let (su, errs) = p("module m; genvar i, j;\nendmodule");
    assert!(errs.is_empty(), "{errs:?}");
    let su = su.unwrap();
    let m = first_module(&su);
    let ModuleItem::Genvar { names, .. } = &m.body[0] else {
        panic!("not a genvar decl: {:?}", m.body[0]);
    };
    assert_eq!(
        names.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
        ["i", "j"]
    );
}

// g2. labeled generate-for with an instance body → For{label hoisted to "g"},
//     init/step lvalue "i", body one Item(Instance).
#[test]
fn g2_gen_for_labeled_instance() {
    let items = gen_of(
        "generate for (i = 0; i < 3; i = i + 1) begin : g\n  leaf u (.a(x[i]));\nend\nendgenerate",
    );
    assert_eq!(items.len(), 1);
    let GenItem::For {
        init,
        step,
        label,
        body,
        ..
    } = &items[0]
    else {
        panic!("not a For: {:?}", items[0]);
    };
    assert_eq!(init.lvalue.name, "i");
    assert_eq!(step.lvalue.name, "i");
    assert_eq!(label.as_ref().map(|l| l.name.as_str()), Some("g"));
    assert_eq!(body.len(), 1);
    assert!(matches!(
        &body[0],
        GenItem::Item(mi) if matches!(**mi, ModuleItem::Instance(_))
    ));
}

// g3. bare-body generate-for (no begin/end) → For{label none}, body one
//     Item(ContAssign).
#[test]
fn g3_gen_for_bare_body() {
    let items = gen_of("generate for (i = 0; i < 2; i = i + 1) assign y[i] = a[i];\nendgenerate");
    assert_eq!(items.len(), 1);
    let GenItem::For { label, body, .. } = &items[0] else {
        panic!("not a For: {:?}", items[0]);
    };
    assert!(label.is_none());
    assert_eq!(body.len(), 1);
    assert!(matches!(
        &body[0],
        GenItem::Item(mi) if matches!(**mi, ModuleItem::ContAssign(_))
    ));
}

// g4. generate-if with and without else.
#[test]
fn g4_gen_if_else() {
    let items = gen_of("generate if (W) assign y = a; else assign y = b;\nendgenerate");
    let GenItem::If { then_b, else_b, .. } = &items[0] else {
        panic!("not an If: {:?}", items[0]);
    };
    assert_eq!(then_b.len(), 1);
    assert_eq!(else_b.len(), 1);

    let items = gen_of("generate if (W) assign y = a;\nendgenerate");
    let GenItem::If { then_b, else_b, .. } = &items[0] else {
        panic!("not an If: {:?}", items[0]);
    };
    assert_eq!(then_b.len(), 1);
    assert!(else_b.is_empty());
}

// g5. generate-case: 0:…  1,2:…  default:… → Match{1}, Match{2}, Default.
#[test]
fn g5_gen_case() {
    let items = gen_of(
            "generate case (W)\n  0: assign y = a;\n  1, 2: assign y = b;\n  default: assign y = c;\nendcase\nendgenerate",
        );
    let GenItem::Case { items: cis, .. } = &items[0] else {
        panic!("not a Case: {:?}", items[0]);
    };
    assert_eq!(cis.len(), 3);
    assert!(matches!(&cis[0], GenCaseItem::Match { labels, .. } if labels.len() == 1));
    assert!(matches!(&cis[1], GenCaseItem::Match { labels, .. } if labels.len() == 2));
    assert!(matches!(&cis[2], GenCaseItem::Default { .. }));
}

// g6. (M2 clamp) truncated generate headers recover with errors and DO NOT
//     panic on the inverted Span::to union.
#[test]
fn g6_truncated_headers_no_panic() {
    for src in [
        "module m; generate for endgenerate\nendmodule",
        "module m; generate if (\nendmodule",
        "module m; generate case (\nendmodule",
        "module m; generate for (\nendmodule",
    ] {
        let (toks, _lex) = hdl_lexer::lex(src);
        let (_su, errs) = parse(&toks, src);
        assert!(!errs.is_empty(), "expected parse errors for `{src}`");
    }
}
