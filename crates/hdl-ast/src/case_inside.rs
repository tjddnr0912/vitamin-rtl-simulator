//! The label shape of a `case (e) inside` item (IEEE 1800-2017 §12.5.4) — its one
//! home, shared by the producer (`hdl-parser`) and the consumer (`elaborate`).
//!
//! A case-inside item is an `open_range_list`: value elements and `[lo : hi]`
//! ranges. Each one is stored as ONE `CaseItem::Match` label in the shape the
//! `inside` operator's desugar already builds, over an inert left operand `P`:
//!
//! | element | label |
//! |---|---|
//! | `v` | `Binary { InsideEq, P, v }` |
//! | `[lo : hi]` | `Binary { LogAnd, Binary { Ge, P, lo }, Binary { Le, P, hi } }` |
//!
//! `P` is a bare `ExprKind::Dollar`. It is never lowered: elaborate takes a label
//! apart by its top-level shape ([`inside_label`]) and reads only `v`, `lo` and
//! `hi`, so the value `P` would have is never asked for. Any lane that lowered a
//! label generically would meet `P` as a stray `$`, which is loud (E3009), never a
//! silent value. Functions only: no type here derives `SchemaHash`, so the `.vu`
//! root is untouched by this module.

use crate::{BinOp, Expr, ExprKind, Span};

/// One element of an `open_range_list`, taken apart from its stored label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsideLabel<'a> {
    /// A value element `v`.
    Value(&'a Expr),
    /// A range element `[lo : hi]`.
    Range(&'a Expr, &'a Expr),
}

fn placeholder(span: Span) -> Expr {
    Expr {
        kind: ExprKind::Dollar,
        span,
    }
}

fn bin(op: BinOp, lhs: Expr, rhs: Expr) -> Expr {
    let span = lhs.span.to(rhs.span);
    Expr {
        kind: ExprKind::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        span,
    }
}

/// The label for a value element `v`. `p_span` is the element's first token, so
/// the label spans the element's text.
pub fn inside_value_label(p_span: Span, v: Expr) -> Expr {
    bin(BinOp::InsideEq, placeholder(p_span), v)
}

/// The label for a range element `[lo : hi]`. `p_span` is the `[` token.
pub fn inside_range_label(p_span: Span, lo: Expr, hi: Expr) -> Expr {
    let ge = bin(BinOp::Ge, placeholder(p_span), lo);
    let le = bin(BinOp::Le, placeholder(p_span), hi);
    bin(BinOp::LogAnd, ge, le)
}

fn is_placeholder(e: &Expr) -> bool {
    matches!(e.kind, ExprKind::Dollar)
}

/// Take a stored label apart. Matches EXACTLY the two shapes the builders above
/// make, with a bare `Dollar` in every `P` position; anything else is `None`, which
/// the consumer reports as an internal error rather than lowering it.
pub fn inside_label(e: &Expr) -> Option<InsideLabel<'_>> {
    let ExprKind::Binary { op, lhs, rhs } = &e.kind else {
        return None;
    };
    match op {
        BinOp::InsideEq if is_placeholder(lhs) => Some(InsideLabel::Value(rhs)),
        BinOp::LogAnd => {
            let (
                ExprKind::Binary {
                    op: BinOp::Ge,
                    lhs: p_lo,
                    rhs: lo,
                },
                ExprKind::Binary {
                    op: BinOp::Le,
                    lhs: p_hi,
                    rhs: hi,
                },
            ) = (&lhs.kind, &rhs.kind)
            else {
                return None;
            };
            (is_placeholder(p_lo) && is_placeholder(p_hi)).then_some(InsideLabel::Range(lo, hi))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::IntLitKind;

    fn lit(raw: &str, lo: u32) -> Expr {
        Expr {
            kind: ExprKind::IntLit {
                kind: IntLitKind::Sized,
                raw: raw.to_string(),
            },
            span: Span::new(lo, lo + raw.len() as u32),
        }
    }

    #[test]
    fn value_and_range_labels_round_trip() {
        let v = lit("4'b1?00", 10);
        let l = inside_value_label(Span::new(10, 11), v.clone());
        assert_eq!(l.span, Span::new(10, 17));
        assert_eq!(inside_label(&l), Some(InsideLabel::Value(&v)));

        let (lo, hi) = (lit("4'd1", 21), lit("4'd3", 26));
        let r = inside_range_label(Span::new(20, 21), lo.clone(), hi.clone());
        assert_eq!(r.span, Span::new(20, 30));
        assert_eq!(inside_label(&r), Some(InsideLabel::Range(&lo, &hi)));
    }

    #[test]
    fn any_other_shape_is_none() {
        let a = lit("4'd1", 0);
        let b = lit("4'd2", 5);
        // a user `a == b`, a user `$ == b` under the wrong operator, and a range
        // whose placeholder is not a bare `$`
        assert_eq!(
            inside_label(&bin(BinOp::InsideEq, a.clone(), b.clone())),
            None
        );
        assert_eq!(
            inside_label(&bin(BinOp::Eq, placeholder(Span::new(0, 1)), b.clone())),
            None
        );
        let bad = bin(
            BinOp::LogAnd,
            bin(BinOp::Ge, a.clone(), b.clone()),
            bin(BinOp::Le, placeholder(Span::new(0, 1)), b.clone()),
        );
        assert_eq!(inside_label(&bad), None);
        assert_eq!(inside_label(&a), None);
    }
}
