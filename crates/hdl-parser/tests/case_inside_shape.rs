//! `case (e) inside` (IEEE 1800-2017 §12.5.4) — the parse shape. Every
//! `open_value_range` becomes one `CaseItem::Match` label in the form
//! `hdl_ast::case_inside` defines over an inert `$` placeholder, and the qualified
//! forms pass through `parse_unique_priority` unchanged. IEEE 1364 does not reserve
//! `inside`, so the word starts a case-inside only after `case` and only when it is not
//! a whole label itself (`inside:`, `inside, …`); `casez` / `casex` never take it, so
//! those parse exactly as before (elaborate refuses the remaining overlap when a name
//! `inside` is declared). A plain `case` parses exactly as before.
use hdl_ast::case_inside::{inside_label, InsideLabel};
use hdl_ast::{CaseItem, CaseKind, Expr, ExprKind, ModuleItem, SourceUnit, Stmt, TopItem};

fn parse(src: &str) -> (Option<SourceUnit>, Vec<hdl_parser::ParseError>) {
    let (toks, le) = hdl_lexer::lex(src);
    assert!(le.is_empty(), "lex errors: {le:?}");
    hdl_parser::parse(&toks, src)
}

/// The statement body of the first procedural block of the first module.
fn first_body(su: &SourceUnit) -> Stmt {
    let m = su
        .items
        .iter()
        .find_map(|i| match i {
            TopItem::Module(m) => Some(m),
            _ => None,
        })
        .expect("a module");
    m.body
        .iter()
        .find_map(|i| match i {
            ModuleItem::Proc(pb) => Some((*pb.body).clone()),
            _ => None,
        })
        .expect("a procedural block")
}

fn text<'a>(src: &'a str, e: &Expr) -> &'a str {
    &src[e.span.lo as usize..e.span.hi as usize]
}

#[test]
fn labels_take_the_shared_shape() {
    let src = "module m; always_comb case (v) inside 4'b1?00, [4'd1:4'd3]: y = 1; \
               default: y = 0; endcase endmodule";
    let (su, errs) = parse(src);
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    let Stmt::Case {
        kind,
        scrutinee,
        items,
        ..
    } = first_body(&su.unwrap())
    else {
        panic!("not a case")
    };
    assert_eq!(kind, CaseKind::Inside);
    assert_eq!(text(src, &scrutinee), "v");
    let CaseItem::Match { labels, .. } = &items[0] else {
        panic!("first item is not a match")
    };
    assert_eq!(labels.len(), 2);
    let Some(InsideLabel::Value(v)) = inside_label(&labels[0]) else {
        panic!("first label is not a value element: {:?}", labels[0])
    };
    assert_eq!(text(src, v), "4'b1?00");
    assert_eq!(text(src, &labels[0]), "4'b1?00");
    let Some(InsideLabel::Range(lo, hi)) = inside_label(&labels[1]) else {
        panic!("second label is not a range element: {:?}", labels[1])
    };
    assert_eq!((text(src, lo), text(src, hi)), ("4'd1", "4'd3"));
    assert_eq!(text(src, &labels[1]), "[4'd1:4'd3]");
    // the placeholder is a bare `$` at the element's first token
    let ExprKind::Binary { lhs, .. } = &labels[0].kind else {
        panic!()
    };
    assert!(matches!(lhs.kind, ExprKind::Dollar));
    assert!(matches!(items[1], CaseItem::Default { .. }));
}

#[test]
fn unique_without_default_gets_the_violation_arm() {
    let src = "module m; always_comb unique case (v) inside 4'd1: y = 1; endcase endmodule";
    let (su, errs) = parse(src);
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    let Stmt::Case { kind, items, .. } = first_body(&su.unwrap()) else {
        panic!("not a case")
    };
    assert_eq!(kind, CaseKind::Inside);
    let Some(CaseItem::Default { body, .. }) = items.last() else {
        panic!("no synthesized default")
    };
    let Stmt::SysTaskCall { name, .. } = &**body else {
        panic!("default is not the violation task: {body:?}")
    };
    assert_eq!(name.name, hdl_ast::UNIQUE_VIOLATION_TASK);

    // unique0 suppresses the no-match report, as for a plain case
    let src0 = "module m; always_comb unique0 case (v) inside 4'd1: y = 1; endcase endmodule";
    let (su0, errs0) = parse(src0);
    assert!(errs0.is_empty(), "parse errors: {errs0:?}");
    let Stmt::Case { items, .. } = first_body(&su0.unwrap()) else {
        panic!("not a case")
    };
    assert_eq!(items.len(), 1);
}

/// `casez` / `casex` never start a case-inside: the word is an ordinary label name,
/// as before the construct existed (a 1364 design may declare `inside`).
#[test]
fn casez_and_casex_read_inside_as_a_label() {
    for kw in ["casez", "casex"] {
        let src = format!(
            "module m; always_comb {kw} (v) inside[2:1]: y = 1; default: y = 0; endcase endmodule"
        );
        let (su, errs) = parse(&src);
        assert!(errs.is_empty(), "{kw}: {errs:?}");
        let Stmt::Case { kind, items, .. } = first_body(&su.unwrap()) else {
            panic!("not a case")
        };
        assert_ne!(kind, CaseKind::Inside);
        let CaseItem::Match { labels, .. } = &items[0] else {
            panic!()
        };
        assert!(
            matches!(labels[0].kind, ExprKind::PartSelect { .. }),
            "{kw}: {labels:?}"
        );
        assert_eq!(text(&src, &labels[0]), "inside[2:1]");
    }
}

/// `inside:` and `inside, 4'd9:` after `case (…)` are labels naming a variable `inside`.
#[test]
fn inside_as_a_whole_label_stays_a_plain_case() {
    let src = "module m; always_comb begin case (v) inside: y = 1; default: y = 0; endcase \
               case (v) inside, 4'd9: y = 2; default: y = 0; endcase end endmodule";
    let (su, errs) = parse(src);
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    let Stmt::Block { stmts, .. } = first_body(&su.unwrap()) else {
        panic!("not a block")
    };
    for (st, n) in stmts.iter().zip([1, 2]) {
        let Stmt::Case { kind, items, .. } = st else {
            panic!("not a case: {st:?}")
        };
        assert_eq!(*kind, CaseKind::Case);
        let CaseItem::Match { labels, .. } = &items[0] else {
            panic!()
        };
        assert_eq!(labels.len(), n);
        assert_eq!(text(src, &labels[0]), "inside");
        assert!(matches!(labels[0].kind, ExprKind::Ident(_)));
    }
}

#[test]
fn plain_case_is_unchanged() {
    let src = "module m; always_comb case (v) 4'd1, 4'd2: y = 1; default: y = 0; endcase endmodule";
    let (su, errs) = parse(src);
    assert!(errs.is_empty(), "parse errors: {errs:?}");
    let Stmt::Case { kind, items, .. } = first_body(&su.unwrap()) else {
        panic!("not a case")
    };
    assert_eq!(kind, CaseKind::Case);
    let CaseItem::Match { labels, .. } = &items[0] else {
        panic!()
    };
    assert!(labels
        .iter()
        .all(|l| matches!(l.kind, ExprKind::IntLit { .. })));
    assert_eq!(text(src, &labels[1]), "4'd2");
}
