//! `ExprKind::IntLit.origin` (§5.2 PARSE-1, the provenance piece) on what the parser
//! writes: a lexer literal is `Source` (`Parser::lit_int`), every number the parser makes
//! is `Parser`, one pin per family of maker. `LitOrigin`'s `PartialEq` is always true, so
//! a pin reads a tag only by pattern (`is_source` / `is_parser`, each with a wrong-variant
//! control). No oracle: the field is not a value of the language, it is which code built
//! the node (`hdl-ast/tests/lit_origin_home.rs` holds the same by path).
use super::composed_member::{no_spans, same, top};
use super::*;

fn is_source(k: &ExprKind) -> bool {
    matches!(
        k,
        ExprKind::IntLit {
            origin: LitOrigin::Source,
            ..
        }
    )
}

fn is_parser(k: &ExprKind) -> bool {
    matches!(
        k,
        ExprKind::IntLit {
            origin: LitOrigin::Parser,
            ..
        }
    )
}

/// Every `IntLit` under `e`, in visit order.
fn leaves(e: &Expr) -> Vec<&ExprKind> {
    let mut out = Vec::new();
    if matches!(e.kind, ExprKind::IntLit { .. }) {
        out.push(&e.kind);
    }
    e.for_each_child(|_, c| out.extend(leaves(c)));
    out
}

/// The leaves are non-empty and all pass `want`.
#[track_caller]
fn all(e: &Expr, want: fn(&ExprKind) -> bool) {
    let l = leaves(e);
    assert!(!l.is_empty() && l.iter().all(|k| want(k)), "{l:?}");
}

/// The right-hand side of `assign x = ..;` after `decls` in a module.
fn rhs(decls: &str, e: &str) -> Expr {
    let src = format!("module m; {decls}\nassign x = {e};\nendmodule");
    let (su, errs) = p(&src);
    assert!(errs.is_empty(), "`{src}`: {errs:?}");
    let su = su.unwrap();
    first_module(&su)
        .body
        .iter()
        .find_map(|i| match i {
            ModuleItem::ContAssign(ca) => Some(ca.assigns[0].1.clone()),
            _ => None,
        })
        .expect("an assign")
}

fn module_items(body: &str) -> Vec<ModuleItem> {
    let src = format!("module m; {body}\nendmodule");
    let (su, errs) = p(&src);
    assert!(errs.is_empty(), "`{src}`: {errs:?}");
    first_module(&su.unwrap()).body.clone()
}

#[test]
fn the_helpers_tell_the_variants_apart() {
    let lexed = expr_of("5");
    let made = rhs("int i;", "$bits(int)");
    assert!(is_source(&lexed.kind) && !is_parser(&lexed.kind));
    assert!(is_parser(&made.kind) && !is_source(&made.kind));
}

#[test]
fn a_lexer_literal_is_source() {
    for e in ["5", "4'b1010", "'hF"] {
        let x = expr_of(e);
        assert!(is_source(&x.kind), "{e}: {:?}", x.kind);
    }
}

/// `$bits(T)` folds at parse time (`expr_primary.rs`, through `Parser::dec_lit`).
#[test]
fn a_folded_bits_is_parser() {
    let e = rhs("typedef logic [5:0] t_t;", "$bits(t_t)");
    assert!(matches!(&e.kind, ExprKind::IntLit { raw, .. } if raw == "6"));
    all(&e, is_parser);
}

/// A `byte` return's range `[7:0]` (`functask.rs`, `dec_range`).
#[test]
fn a_keyword_range_is_parser() {
    let items = module_items("function byte f(input int a); f = a; endfunction");
    let r = items
        .iter()
        .find_map(|i| match i {
            ModuleItem::Func(f) => f.dtype.range.clone(),
            _ => None,
        })
        .expect("a return range");
    assert!(is_parser(&r.msb.kind) && is_parser(&r.lsb.kind), "{r:?}");
}

/// An enum's `.first()` is its first label's value (`enums.rs`, `i64_lit`); the label's
/// own `3` stays the lexer's.
#[test]
fn an_enum_method_constant_is_parser() {
    let e = rhs("typedef enum {A=3, B} e_t; e_t v;", "v.first()");
    assert!(matches!(&e.kind, ExprKind::IntLit { raw, .. } if raw == "3"));
    all(&e, is_parser);
}

/// `i++`'s step (`stmt.rs`, a direct construction).
#[test]
fn an_increment_step_is_parser() {
    let pb = proc_of("int i; initial i++;");
    let Stmt::Blocking { rhs, .. } = &*pb.body else {
        panic!("{:?}", pb.body);
    };
    assert!(matches!(&rhs.kind, ExprKind::Binary { op: BinOp::Add, .. }));
    all(rhs, is_parser);
}

/// A multi-dimensional packed parameter: its flat range (part 5's respell builders,
/// `sub` / `dec_lit`) and a select on it (`packed_md.rs`, `lit`).
#[test]
fn a_respelled_bound_and_select_are_parser() {
    let items = module_items("parameter logic [3:0][1:0] P = 0;");
    let r = items
        .iter()
        .find_map(|i| match i {
            ModuleItem::Param(d) => d.dtype.range.clone(),
            _ => None,
        })
        .expect("a flat range");
    assert!(matches!(&r.msb.kind, ExprKind::IntLit { raw, .. } if raw == "7"));
    assert!(is_parser(&r.msb.kind) && is_parser(&r.lsb.kind), "{r:?}");
    all(&rhs("parameter logic [3:0][1:0] P = 0;", "P[2]"), is_parser);
}

/// `'{default: '1}` into a packed struct: each member's fill at its width
/// (`assign_pattern.rs`, `fill_at_width`, `Sized`).
#[test]
fn a_fill_at_width_is_parser() {
    let items = module_items(
        "typedef struct packed { logic [3:0] a; logic b; } s_t; s_t s;\nassign s = '{default: '1};",
    );
    let e = items
        .iter()
        .find_map(|i| match i {
            ModuleItem::ContAssign(ca) => Some(ca.assigns[0].1.clone()),
            _ => None,
        })
        .expect("an assign");
    let l = leaves(&e);
    assert!(
        l.iter().any(|k| matches!(
            k,
            ExprKind::IntLit {
                kind: IntLitKind::Sized,
                origin: LitOrigin::Parser,
                ..
            }
        )),
        "{l:?}"
    );
    assert!(l.iter().all(|k| is_parser(k)), "{l:?}");
}

/// A class value parameter's actual replaces the name (`monomorph.rs` `subst_expr`): the
/// node is the actual's clone, so it stays `Source` at the NAME's span. `Source` says
/// "lexed", not "written here" (`LitOrigin`'s doc).
#[test]
fn source_survives_substitution() {
    let actual = expr_of("5");
    let name_span = Span::new(40, 41);
    let mut e = Expr {
        kind: ExprKind::Ident(HierPath {
            segments: vec![Ident {
                name: "N".to_string(),
                span: name_span,
            }],
            span: name_span,
        }),
        span: name_span,
    };
    let map = std::collections::BTreeMap::from([("N".to_string(), actual.clone())]);
    crate::monomorph::subst_expr(&mut e, &map);
    assert!(is_source(&e.kind), "{:?}", e.kind);
    assert_eq!(e.span, name_span);
    assert_ne!(actual.span, name_span);
}

#[test]
fn a_clone_keeps_the_origin() {
    let made = rhs("typedef logic [5:0] t_t;", "$bits(t_t)");
    assert!(is_parser(&made.clone().kind));
    assert!(is_source(&expr_of("5").clone().kind));
}

/// `no_spans` (composed_member.rs) erases exactly `, origin: <V> }` outside quoted text
/// and nothing a compare needs (§5.2 PARSE-1 difference 7).
#[test]
fn no_spans_erases_the_origin_only() {
    let lit = |kind: IntLitKind, raw: &str| ExprKind::IntLit {
        kind,
        raw: raw.to_string(),
        origin: LitOrigin::Parser,
    };
    let src_lit = ExprKind::IntLit {
        kind: IntLitKind::Decimal,
        raw: "1".to_string(),
        origin: LitOrigin::Source,
    };
    let t = |k: &ExprKind| no_spans(&format!("{k:?}"));
    // a print with literals keeps no `origin`
    let m = t(&rhs("typedef logic [5:0] t_t;", "4'b1010 + $bits(t_t)").kind);
    assert!(
        !m.contains("origin") && m.contains("raw: \"4'b1010\""),
        "{m}"
    );
    // two tags erase to one text ; two raws and two kinds still differ
    assert_eq!(t(&src_lit), t(&lit(IntLitKind::Decimal, "1")));
    assert_ne!(
        t(&lit(IntLitKind::Decimal, "1")),
        t(&lit(IntLitKind::Decimal, "2"))
    );
    assert_ne!(
        t(&lit(IntLitKind::Decimal, "1")),
        t(&lit(IntLitKind::Sized, "1"))
    );
    // a string literal holding the same characters is untouched
    let s = ExprKind::StrLit {
        raw: "\"a, origin: Parser }\"".to_string(),
    };
    assert_eq!(t(&s), format!("{s:?}"));
    assert!(t(&s).contains(", origin: Parser }"));
}

/// Two of composed_member's pairs: one tree spans aside (`same`), the natural select's
/// bounds made by the parser, the hand form's lexed.
#[test]
fn a_composed_pair_differs_only_in_origin() {
    for (natural, hand) in [
        ("assign x = s.m[2].e;", "assign x = s[9:9];"),
        ("assign d[1:0] = s.m[3].q;", "assign d[1:0] = s[14:13];"),
    ] {
        same(natural, hand);
        let rhs_of = |body: &str| {
            let (su, errs) = p(&top(body));
            assert!(errs.is_empty(), "{errs:?}");
            match su.unwrap().items.last() {
                Some(TopItem::Module(m)) => m
                    .body
                    .iter()
                    .find_map(|i| match i {
                        ModuleItem::ContAssign(ca) => Some(ca.assigns[0].1.clone()),
                        _ => None,
                    })
                    .expect("an assign"),
                _ => panic!("no module"),
            }
        };
        all(&rhs_of(natural), is_parser);
        all(&rhs_of(hand), is_source);
    }
}
