//! `ExprKind::IntLit.origin` (§5.2 PARSE-1, the provenance piece): a literal elaborate
//! builds is `Elab` (`hdl-ast/tests/lit_origin_home.rs` holds every one of the 18 by
//! path). Read by pattern only: `LitOrigin`'s `PartialEq` is always true. No oracle: the
//! field is which code built the node, not a value of the language.
use super::*;

#[test]
fn an_elaborate_literal_is_elab() {
    let one = sva_one(SP);
    let pos = cov_int_lit_pos(5, SP);
    for e in [&one, &pos] {
        assert!(matches!(
            e.kind,
            ast::ExprKind::IntLit {
                origin: LitOrigin::Elab,
                ..
            }
        ));
        assert!(!matches!(
            e.kind,
            ast::ExprKind::IntLit {
                origin: LitOrigin::Parser,
                ..
            }
        ));
    }
}
