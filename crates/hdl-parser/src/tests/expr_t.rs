use super::*;

// 1. mul binds tighter than add:  a + b * c  =>  +(a, *(b,c))
#[test]
fn t1_mul_tighter_than_add() {
    let (op, _l, r) = {
        let e = expr_of("a + b * c");
        let (o, l, r) = bin(&e);
        (o, l.clone(), r.clone())
    };
    assert_eq!(op, BinOp::Add);
    assert_eq!(bin(&r).0, BinOp::Mul);
}

// 2. ternary right-assoc:  a ? b : c ? d : e  =>  a ? b : (c ? d : e)
#[test]
fn t2_ternary_right_assoc() {
    let e = expr_of("a ? b : c ? d : e");
    let ExprKind::Ternary { else_e, .. } = &e.kind else {
        panic!()
    };
    assert!(matches!(else_e.kind, ExprKind::Ternary { .. }));
}

// 3. concat LHS + left-assoc add:  assign {cout,sum} = a + b + cin;
#[test]
fn t3_concat_lhs_left_assoc() {
    let (su, errs) = p("module m; assign {cout, sum} = a + b + cin;\nendmodule");
    assert!(errs.is_empty(), "{errs:?}");
    let su = su.unwrap();
    let m = first_module(&su);
    let ModuleItem::ContAssign(ca) = &m.body[0] else {
        panic!()
    };
    let Lvalue::Concat { parts, .. } = &ca.assigns[0].0 else {
        panic!("LHS not concat")
    };
    assert_eq!(parts.len(), 2);
    let (op, l, _r) = bin(&ca.assigns[0].1);
    assert_eq!(op, BinOp::Add);
    assert_eq!(bin(l).0, BinOp::Add); // left child is (a+b)  → left-assoc
}

// 4. ANSI #(param)(ports) + direction inheritance
#[test]
fn t4_ansi_header() {
    let (su, errs) = p("module adder #(parameter WIDTH = 8)\
            (input [WIDTH-1:0] a, b, output [WIDTH-1:0] sum);\nendmodule");
    assert!(errs.is_empty(), "{errs:?}");
    let su = su.unwrap();
    let m = first_module(&su);
    assert_eq!(m.name.name, "adder");
    assert_eq!(m.params.len(), 1);
    assert_eq!(m.params[0].kind, ParamKind::Parameter);
    let PortList::Ansi(ports) = &m.ports else {
        panic!("not ANSI")
    };
    assert_eq!(ports.len(), 3);
    assert_eq!(ports[0].dir, PortDir::Input);
    assert_eq!(ports[1].dir, PortDir::Input); // `b` inherits
    assert_eq!(ports[2].dir, PortDir::Output);
}

// 5. non-ANSI module: header names + body dir/type
#[test]
fn t5_non_ansi() {
    let (su, errs) = p(
        "module m(a, b, y);\n  input a, b;\n  output y;\n  wire [3:0] tmp;\n\
            assign y = a & b;\nendmodule",
    );
    assert!(errs.is_empty(), "{errs:?}");
    let su = su.unwrap();
    let m = first_module(&su);
    let PortList::NonAnsi(names) = &m.ports else {
        panic!("not non-ANSI")
    };
    assert_eq!(
        names.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
        ["a", "b", "y"]
    );
    assert!(matches!(m.body[0], ModuleItem::PortDecl(_)));
    assert!(m.body.iter().any(|i| matches!(i, ModuleItem::NetVar(_))));
    assert!(m
        .body
        .iter()
        .any(|i| matches!(i, ModuleItem::ContAssign(_))));
}

// 6. vector range is an expr, not pre-evaluated:  wire [WIDTH-1:0] bus;
#[test]
fn t6_range_is_expr() {
    let (su, _e) = p("module m; wire [WIDTH-1:0] bus;\nendmodule");
    let su = su.unwrap();
    let m = first_module(&su);
    let ModuleItem::NetVar(nv) = &m.body[0] else {
        panic!()
    };
    let r = nv.range.as_ref().unwrap();
    assert_eq!(bin(&r.msb).0, BinOp::Sub);
    assert!(matches!(r.lsb.kind, ExprKind::IntLit { .. }));
}

// 7. indexed part-select [b+:w]
#[test]
fn t7_indexed_part_select() {
    let e = expr_of("data[base +: 8]");
    let ExprKind::IndexedPart { dir, .. } = &e.kind else {
        panic!("{:?}", e.kind)
    };
    assert_eq!(*dir, PartDir::PlusColon);
}

// 8. & tighter than | :  a & b | c  =>  |(&(a,b), c)
#[test]
fn t8_and_tighter_than_or() {
    let e = expr_of("a & b | c");
    let (op, l, _r) = bin(&e);
    assert_eq!(op, BinOp::BitOr);
    assert_eq!(bin(l).0, BinOp::BitAnd);
}

// 9. unary tighter than equality:  !a == b  =>  ==(!a, b)
#[test]
fn t9_unary_tighter_than_eq() {
    let e = expr_of("!a == b");
    let (op, l, _r) = bin(&e);
    assert_eq!(op, BinOp::Eq);
    assert!(matches!(
        l.kind,
        ExprKind::Unary {
            op: UnOp::LogNot,
            ..
        }
    ));
}

// 10. add tighter than shift (the doc's #1 gotcha):  a + b << 2  =>  (a+b) << 2
#[test]
fn t10_add_tighter_than_shift() {
    let e = expr_of("a + b << 2");
    let (op, l, _r) = bin(&e);
    assert_eq!(op, BinOp::Shl);
    assert_eq!(bin(l).0, BinOp::Add);
}

// 11. replication value is a Vec, NOT a Concat wrapper (verdict M5):  {3{a}}
#[test]
fn t11_replication_value_is_vec() {
    let e = expr_of("{3{a}}");
    let ExprKind::Replicate { count, value } = &e.kind else {
        panic!("{:?}", e.kind)
    };
    assert!(matches!(count.kind, ExprKind::IntLit { .. }));
    assert_eq!(value.len(), 1);
    assert!(matches!(value[0].kind, ExprKind::Ident(_))); // bare `a`, not Concat{[a]}
}

// 12. mintypmax delay (verdict M2):  assign #(1:2:3) y = a;
#[test]
fn t12_mintypmax_delay() {
    let (su, errs) = p("module m; assign #(1:2:3) y = a;\nendmodule");
    assert!(errs.is_empty(), "{errs:?}");
    let su = su.unwrap();
    let m = first_module(&su);
    let ModuleItem::ContAssign(ca) = &m.body[0] else {
        panic!()
    };
    let d = ca.delay.as_ref().unwrap();
    assert_eq!(d.values.len(), 1);
    assert!(matches!(d.values[0].kind, ExprKind::MinTypMax { .. }));
}

// 13. recovery continues after a bad item (uses a lexer-error token `@`-stray
//     plus garbage); the trailing valid assign still parses (verdict B3).
#[test]
fn t13_recovery_continues() {
    let (su, errs) = p("module m; wire @ ; assign y = a;\nendmodule");
    assert!(!errs.is_empty(), "expected a recovered error");
    let su = su.unwrap();
    let m = first_module(&su);
    assert!(
        m.body
            .iter()
            .any(|i| matches!(i, ModuleItem::ContAssign(_))),
        "parser must recover and parse the trailing assign"
    );
}

// 14. termination edges (verdict H3-soundness): must not hang / must terminate.
#[test]
fn t14_termination_edges() {
    assert_eq!(p("").0, None); // empty input ⇒ (None, [])
    let _ = p("module"); // truncated header
    let _ = p("module module;"); // sync-anchor == entry-token trap
    let _ = p("module m; endmodule extra ;"); // trailing junk
                                              // reaching here without hang is the assertion
}

// 15. ** LEFT-assoc and unary precedence:  -a ** b  =>  (-a) ** b ; 2**3**4
//     => (2**3)**4 (IEEE Table 11-2 / iverilog).
#[test]
fn t15_pow_assoc_and_unary() {
    let e = expr_of("2 ** 3 ** 4");
    let (op, l, _r) = bin(&e);
    assert_eq!(op, BinOp::Pow);
    assert_eq!(bin(&l.clone()).0, BinOp::Pow); // LEFT child is 2**3 (left-assoc)
    let e2 = expr_of("- a ** b");
    let (op2, l2, _r2) = bin(&e2);
    assert_eq!(op2, BinOp::Pow); // top is **
    assert!(matches!(
        l2.kind,
        ExprKind::Unary {
            op: UnOp::Minus,
            ..
        }
    )); // left is (-a)
}
