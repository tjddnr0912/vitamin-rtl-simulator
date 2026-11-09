//! Contract pin of `value_rules`' lanes (ROADMAP §5.2 PARSE-1): each answer below is PRE's,
//! written literally. A value row that changes a lane (widens an op set, moves a cutoff) edits
//! the kernel and this pin together.

use hdl_ast::value_rules::{binop_in, clog2_u64, ctx5_binop, decimal_i64, wrap_i128, OpSet};
use hdl_ast::BinOp;

/// Every `BinOp` variant.
const ALL: [BinOp; 27] = [
    BinOp::Add,
    BinOp::Sub,
    BinOp::Mul,
    BinOp::Div,
    BinOp::Mod,
    BinOp::Pow,
    BinOp::Shl,
    BinOp::Shr,
    BinOp::AShl,
    BinOp::AShr,
    BinOp::Lt,
    BinOp::Le,
    BinOp::Gt,
    BinOp::Ge,
    BinOp::Eq,
    BinOp::Ne,
    BinOp::CaseEq,
    BinOp::CaseNe,
    BinOp::WildEq,
    BinOp::WildNe,
    BinOp::BitAnd,
    BinOp::BitXor,
    BinOp::BitXnor,
    BinOp::BitOr,
    BinOp::LogAnd,
    BinOp::LogOr,
    BinOp::InsideEq,
];

/// Fails to compile when `BinOp` gains a variant, so `ALL` is every variant.
#[allow(dead_code)]
fn all_is_exhaustive(op: BinOp) {
    match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow => {}
        BinOp::Shl | BinOp::Shr | BinOp::AShl | BinOp::AShr => {}
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne => {}
        BinOp::CaseEq | BinOp::CaseNe | BinOp::WildEq | BinOp::WildNe | BinOp::InsideEq => {}
        BinOp::BitAnd | BinOp::BitXor | BinOp::BitXnor | BinOp::BitOr => {}
        BinOp::LogAnd | BinOp::LogOr => {}
    }
}

#[test]
fn op_sets_admit_exactly_their_operators() {
    for op in ALL {
        assert_eq!(
            OpSet::Parser4.admits(op),
            matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div),
            "Parser4 {op:?}"
        );
        assert_eq!(
            OpSet::Lit3.admits(op),
            matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul),
            "Lit3 {op:?}"
        );
    }
}

/// (a, b, Add, Sub, Mul, Div): PRE's checked i64 answers.
type PairRow = (i64, i64, Option<i64>, Option<i64>, Option<i64>, Option<i64>);

#[test]
fn binop_in_answers_pre_per_set() {
    const MAX: i64 = i64::MAX;
    const MIN: i64 = i64::MIN;
    // (a, b, Add, Sub, Mul, Div): PRE's checked i64 answers.
    #[rustfmt::skip]
    let rows: [PairRow; 6] = [
        (7, 2, Some(9), Some(5), Some(14), Some(3)),
        (7, 0, Some(7), Some(7), Some(0), None),
        (MAX, 1, None, Some(MAX - 1), Some(MAX), Some(MAX)),
        (MIN, -1, None, Some(MIN + 1), None, None),
        (-8, 1, Some(-7), Some(-9), Some(-8), Some(-8)),
        (3, 64, Some(67), Some(-61), Some(192), Some(0)),
    ];
    for (a, b, add, sub, mul, div) in rows {
        for op in ALL {
            let pre = match op {
                BinOp::Add => add,
                BinOp::Sub => sub,
                BinOp::Mul => mul,
                BinOp::Div => div,
                _ => None,
            };
            let lit3 = if matches!(op, BinOp::Div) { None } else { pre };
            assert_eq!(
                binop_in(OpSet::Parser4, op, a, b),
                pre,
                "Parser4 {op:?} ({a}, {b})"
            );
            assert_eq!(
                binop_in(OpSet::Lit3, op, a, b),
                lit3,
                "Lit3 {op:?} ({a}, {b})"
            );
        }
    }
    // `**` is outside both sets: declined before `const_binop`'s debug_assert arm.
    assert_eq!(binop_in(OpSet::Parser4, BinOp::Pow, 2, 3), None);
    assert_eq!(binop_in(OpSet::Lit3, BinOp::Pow, 2, 3), None);
}

#[test]
fn ctx5_binop_is_unmasked() {
    #[rustfmt::skip]
    let rows: [(BinOp, i128, i128, u32, bool, Option<i128>); 13] = [
        (BinOp::Add, 15, 1, 4, false, Some(16)),
        (BinOp::Sub, 0, 1, 4, false, Some(-1)),
        (BinOp::Mul, 15, 15, 4, true, Some(1)),
        (BinOp::Mul, 15, 15, 4, false, Some(225)),
        (BinOp::Div, 14, 4, 4, true, Some(0)),
        (BinOp::Mod, 9, 0, 4, false, None),
        (BinOp::Shl, 1, 1, 4, false, None),
        (BinOp::Add, (1 << 32) - 1, 1, 32, false, Some(1 << 32)),
        (BinOp::Mod, 7, 3, 32, false, Some(1)),
        (BinOp::Div, (1 << 32) - 2, 2, 32, true, Some(-1)),
        (BinOp::Sub, 0, 1, 64, true, Some(-1)),
        (BinOp::Mul, (1 << 64) - 1, (1 << 64) - 1, 64, true, Some(1)),
        (BinOp::Div, 1 << 63, 2, 64, true, Some(-(1 << 62))),
    ];
    for (op, x, y, w, s, want) in rows {
        assert_eq!(
            ctx5_binop(op, x, y, w, s),
            want,
            "{op:?} {x} {y} w={w} signed={s}"
        );
    }
}

#[test]
fn clog2_u64_edges() {
    let rows: [(u64, u32); 9] = [
        (0, 0),
        (1, 0),
        (2, 1),
        (3, 2),
        (4, 2),
        (5, 3),
        (1 << 32, 32),
        ((1 << 32) + 1, 33),
        (u64::MAX, 64),
    ];
    for (n, want) in rows {
        assert_eq!(clog2_u64(n), want, "clog2_u64({n})");
    }
}

#[test]
fn decimal_i64_edges() {
    let rows: [(&str, Option<i64>); 9] = [
        ("0", Some(0)),
        ("1_0", Some(10)),
        ("+5", Some(5)),
        ("-0", Some(0)),
        ("9223372036854775807", Some(i64::MAX)),
        ("9223372036854775808", None),
        ("", None),
        ("_", None),
        (" 5", None),
    ];
    for (s, want) in rows {
        assert_eq!(decimal_i64(s), want, "decimal_i64({s:?})");
    }
}

#[test]
fn wrap_i128_edges() {
    let m64 = i128::from(u64::MAX);
    let rows: [(i128, u32, bool, Option<i64>); 10] = [
        (16, 4, false, Some(0)),
        (15, 4, true, Some(-1)),
        (5, 0, false, None),
        (5, 0, true, None),
        (5, 65, false, None),
        (5, 65, true, None),
        (m64, 64, false, None),
        (m64, 64, true, Some(-1)),
        (m64 - 4, 64, true, Some(-5)),
        (m64 - 4, 64, false, None),
    ];
    for (v, w, s, want) in rows {
        assert_eq!(wrap_i128(v, w, s), want, "wrap_i128({v}, {w}, {s})");
    }
}
