//! `Expr::for_each_child` / `Expr::regime` against a hand-written table: one row per
//! `ExprKind` variant (plus the edge shapes a walker must not get wrong — an empty
//! list, an absent optional child, the operator- and name-dependent classes), each
//! row naming every child in visit order with its `ChildPos`.
//!
//! No tool oracle: the table is the specification of the AST's child structure, and
//! the evaluation classes are IEEE 1800 §11.4.7 (`&&`/`||`), §11.4.11 (`?:`) and
//! §20.5/§20.6 (`$bits` and the array queries do not evaluate their operand). The
//! visit order is the left-to-right order the general hoister measured against
//! iverilog (`elaborate`'s `hoist::Shape`).
//!
//! A new variant fails to compile `variant_name` below until it gets a row.

use hdl_ast::walk::{ChildPos, Regime};
use hdl_ast::*;
use std::collections::BTreeSet;

fn sp() -> Span {
    Span::new(0, 0)
}
fn id(n: &str) -> Ident {
    Ident {
        name: n.to_string(),
        span: sp(),
    }
}
fn path(segs: &[&str]) -> HierPath {
    HierPath {
        segments: segs.iter().map(|s| id(s)).collect(),
        span: sp(),
    }
}
fn ex(kind: ExprKind) -> Expr {
    Expr { kind, span: sp() }
}
/// A named leaf: the table identifies each child by this name.
fn l(n: &str) -> Expr {
    ex(ExprKind::Ident(path(&[n])))
}
fn b(n: &str) -> Box<Expr> {
    Box::new(l(n))
}

/// Exhaustive on purpose (no wildcard): a new variant does not compile here.
fn variant_name(k: &ExprKind) -> &'static str {
    use ExprKind as K;
    match k {
        K::IntLit { .. } => "IntLit",
        K::RealLit { .. } => "RealLit",
        K::StrLit { .. } => "StrLit",
        K::PkgScoped { .. } => "PkgScoped",
        K::Ident(_) => "Ident",
        K::Unary { .. } => "Unary",
        K::Binary { .. } => "Binary",
        K::Ternary { .. } => "Ternary",
        K::BitSelect { .. } => "BitSelect",
        K::PartSelect { .. } => "PartSelect",
        K::IndexedPart { .. } => "IndexedPart",
        K::Concat { .. } => "Concat",
        K::Replicate { .. } => "Replicate",
        K::Call { .. } => "Call",
        K::RandomizeWith(_) => "RandomizeWith",
        K::ArrayMethodWith(_) => "ArrayMethodWith",
        K::SysCall { .. } => "SysCall",
        K::Paren { .. } => "Paren",
        K::MinTypMax { .. } => "MinTypMax",
        K::New { .. } => "New",
        K::ClassNew { .. } => "ClassNew",
        K::Null => "Null",
        K::TimeLit { .. } => "TimeLit",
        K::NamedArg { .. } => "NamedArg",
        K::MethodCall { .. } => "MethodCall",
        K::Dollar => "Dollar",
        K::Dist { .. } => "Dist",
        K::Cast { .. } => "Cast",
        K::AssignPattern(_) => "AssignPattern",
        K::AssignPatternKeyed(_) => "AssignPatternKeyed",
        K::Error => "Error",
    }
}
const VARIANTS: usize = 31;

struct Row {
    e: Expr,
    kids: &'static [(ChildPos, &'static str)],
    regime: Regime,
}

fn row(kind: ExprKind, kids: &'static [(ChildPos, &'static str)], regime: Regime) -> Row {
    Row {
        e: ex(kind),
        kids,
        regime,
    }
}

fn table() -> Vec<Row> {
    use ChildPos::*;
    use ExprKind as K;
    let u = Regime::Uncond;
    vec![
        row(
            K::IntLit {
                kind: IntLitKind::Sized,
                raw: "4'b1010".into(),
            },
            &[],
            u,
        ),
        row(
            K::RealLit {
                kind: RealLitKind::Fixed,
                raw: "1.5".into(),
            },
            &[],
            u,
        ),
        row(
            K::StrLit {
                raw: "\"s\"".into(),
            },
            &[],
            u,
        ),
        row(
            K::PkgScoped {
                pkg: id("pkg"),
                name: id("p"),
            },
            &[],
            u,
        ),
        row(K::Ident(path(&["a", "b"])), &[], u),
        row(K::Null, &[], u),
        row(K::Dollar, &[], u),
        row(K::Error, &[], u),
        row(
            K::Unary {
                op: UnOp::Minus,
                operand: b("x"),
            },
            &[(Uncond, "x")],
            u,
        ),
        row(
            K::Binary {
                op: BinOp::Add,
                lhs: b("a"),
                rhs: b("b"),
            },
            &[(Uncond, "a"), (Uncond, "b")],
            u,
        ),
        row(
            K::Binary {
                op: BinOp::LogAnd,
                lhs: b("a"),
                rhs: b("b"),
            },
            &[(ShortCircuitLhs, "a"), (ShortCircuitRhs, "b")],
            Regime::ShortCircuit { short_on: false },
        ),
        row(
            K::Binary {
                op: BinOp::LogOr,
                lhs: b("a"),
                rhs: b("b"),
            },
            &[(ShortCircuitLhs, "a"), (ShortCircuitRhs, "b")],
            Regime::ShortCircuit { short_on: true },
        ),
        row(
            K::Ternary {
                cond: b("c"),
                then_e: b("t"),
                else_e: b("e"),
            },
            &[(TernaryCond, "c"), (TernaryThen, "t"), (TernaryElse, "e")],
            Regime::Ternary,
        ),
        row(
            K::BitSelect {
                base: b("v"),
                index: b("i"),
            },
            &[(Uncond, "v"), (Uncond, "i")],
            u,
        ),
        row(
            K::PartSelect {
                base: b("v"),
                msb: b("m"),
                lsb: b("l"),
            },
            &[(Uncond, "v"), (Uncond, "m"), (Uncond, "l")],
            u,
        ),
        row(
            K::IndexedPart {
                base: b("v"),
                offset: b("o"),
                width: b("w"),
                dir: PartDir::MinusColon,
            },
            &[(Uncond, "v"), (Uncond, "o"), (Uncond, "w")],
            u,
        ),
        row(K::Concat { parts: vec![] }, &[], u),
        row(
            K::Concat {
                parts: vec![l("p0"), l("p1"), l("p2")],
            },
            &[(Uncond, "p0"), (Uncond, "p1"), (Uncond, "p2")],
            u,
        ),
        row(
            K::Replicate {
                count: b("n"),
                value: vec![l("x"), l("y")],
            },
            &[(Uncond, "n"), (Uncond, "x"), (Uncond, "y")],
            u,
        ),
        row(
            K::Call {
                name: path(&["f"]),
                args: vec![],
            },
            &[],
            u,
        ),
        row(
            K::Call {
                name: path(&["f"]),
                args: vec![l("a0"), l("a1")],
            },
            &[(Uncond, "a0"), (Uncond, "a1")],
            u,
        ),
        row(
            K::RandomizeWith(Box::new(RandomizeWithExpr {
                name: path(&["obj", "randomize"]),
                args: vec![],
                constraints: vec![],
            })),
            &[],
            Regime::NoHoist,
        ),
        row(
            K::RandomizeWith(Box::new(RandomizeWithExpr {
                name: path(&["obj", "randomize"]),
                args: vec![l("a0")],
                constraints: vec![l("k0"), l("k1")],
            })),
            &[(NoHoist, "a0"), (NoHoist, "k0"), (NoHoist, "k1")],
            Regime::NoHoist,
        ),
        row(
            K::ArrayMethodWith(Box::new(ArrayMethodWithExpr {
                recv: path(&["q"]),
                method: id("sum"),
                iter_var: Some(id("it")),
                with_expr: l("w"),
            })),
            &[(NoHoist, "w")],
            Regime::NoHoist,
        ),
        row(
            K::SysCall {
                name: id("$display"),
                args: vec![l("a0"), l("a1")],
            },
            &[(Uncond, "a0"), (Uncond, "a1")],
            u,
        ),
        row(
            K::SysCall {
                name: id("$clog2"),
                args: vec![l("a0")],
            },
            &[(Uncond, "a0")],
            u,
        ),
        row(
            K::SysCall {
                name: id("$bits"),
                args: vec![l("a0")],
            },
            &[(Unevaluated, "a0")],
            Regime::Unevaluated,
        ),
        row(
            K::SysCall {
                name: id("$size"),
                args: vec![],
            },
            &[],
            Regime::Unevaluated,
        ),
        row(K::Paren { inner: b("x") }, &[(Uncond, "x")], u),
        row(
            K::MinTypMax {
                min: b("mn"),
                typ: b("ty"),
                max: b("mx"),
            },
            &[(NoHoist, "mn"), (NoHoist, "ty"), (NoHoist, "mx")],
            Regime::NoHoist,
        ),
        row(
            K::New {
                size: b("n"),
                src: None,
            },
            &[(Uncond, "n")],
            u,
        ),
        row(
            K::New {
                size: b("n"),
                src: Some(b("s")),
            },
            &[(Uncond, "n"), (Uncond, "s")],
            u,
        ),
        row(
            K::ClassNew {
                args: vec![l("a0"), l("a1")],
            },
            &[(Uncond, "a0"), (Uncond, "a1")],
            u,
        ),
        row(
            K::TimeLit {
                num: b("n"),
                unit_exp: -9,
            },
            &[(Uncond, "n")],
            u,
        ),
        row(
            K::NamedArg {
                formal: id("f"),
                value: None,
            },
            &[],
            u,
        ),
        row(
            K::NamedArg {
                formal: id("f"),
                value: Some(b("v")),
            },
            &[(Uncond, "v")],
            u,
        ),
        row(
            K::MethodCall {
                recv: b("r"),
                method: id("substr"),
                args: vec![l("a0"), l("a1")],
            },
            &[(Uncond, "r"), (Uncond, "a0"), (Uncond, "a1")],
            u,
        ),
        row(
            K::Dist {
                value: b("v"),
                items: vec![
                    DistItem {
                        lo: b("lo0"),
                        hi: None,
                        weight: b("w0"),
                        per_range: false,
                    },
                    DistItem {
                        lo: b("lo1"),
                        hi: Some(b("hi1")),
                        weight: b("w1"),
                        per_range: true,
                    },
                ],
            },
            &[
                (NoHoist, "v"),
                (NoHoist, "lo0"),
                (NoHoist, "w0"),
                (NoHoist, "lo1"),
                (NoHoist, "hi1"),
                (NoHoist, "w1"),
            ],
            Regime::NoHoist,
        ),
        row(
            K::Cast {
                target: CastTarget::Prim(CastPrim::Int),
                expr: b("x"),
            },
            &[(Uncond, "x")],
            u,
        ),
        row(
            K::Cast {
                target: CastTarget::Size(b("n")),
                expr: b("x"),
            },
            &[(Uncond, "n"), (Uncond, "x")],
            u,
        ),
        row(
            K::Cast {
                target: CastTarget::SigningParam {
                    shape_param: id("T$s"),
                },
                expr: b("x"),
            },
            &[(Uncond, "x")],
            u,
        ),
        row(
            K::AssignPattern(vec![l("p0"), l("p1")]),
            &[(Uncond, "p0"), (Uncond, "p1")],
            u,
        ),
        row(
            K::AssignPatternKeyed(vec![
                (AssignPatternKey::Default, l("d")),
                (AssignPatternKey::Member("m".into()), l("v")),
            ]),
            &[(Uncond, "d"), (Uncond, "v")],
            u,
        ),
    ]
}

#[test]
fn every_kind_lists_its_children_in_order_with_positions() {
    let mut covered = BTreeSet::new();
    for r in table() {
        covered.insert(variant_name(&r.e.kind));
        let mut got: Vec<(ChildPos, String)> = Vec::new();
        r.e.for_each_child(|pos, c| {
            let ExprKind::Ident(p) = &c.kind else {
                panic!("table children are named leaves, got {c:?}");
            };
            got.push((pos, p.segments[0].name.clone()));
        });
        let want: Vec<(ChildPos, String)> =
            r.kids.iter().map(|&(p, n)| (p, n.to_string())).collect();
        assert_eq!(got, want, "children of {:?}", r.e.kind);
        assert_eq!(r.e.regime(), r.regime, "regime of {:?}", r.e.kind);
    }
    assert_eq!(covered.len(), VARIANTS, "a variant has no row: {covered:?}");
}

/// A child is visited once per occurrence, and a leaf child's own walk visits nothing:
/// the enumeration is one level deep.
#[test]
fn enumeration_is_one_level_deep() {
    let inner = ex(ExprKind::Binary {
        op: BinOp::Add,
        lhs: b("a"),
        rhs: b("b"),
    });
    let outer = ex(ExprKind::Paren {
        inner: Box::new(inner),
    });
    let mut n = 0;
    outer.for_each_child(|_, c| {
        n += 1;
        assert!(matches!(c.kind, ExprKind::Binary { .. }));
    });
    assert_eq!(n, 1);
}
