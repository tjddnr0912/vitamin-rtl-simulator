//! `case (e) inside` (IEEE 1800-2017 §12.5.4): each item is compared with the
//! set-membership `inside` operator (§11.4.13), so an x/z/? bit of a constant item
//! is a don't-care, a range is `lo <= e && e <= hi`, and the first item that
//! answers `1'b1` takes its arm. The case expression is evaluated once, before the
//! items, through the same capture plain `case` uses (`hoist_case_scrutinee`).
//!
//! Correct-or-loud, by a POSITIVE accept set (`inside_refusal`). §12.5.4 says each
//! item is compared "using the inside operator", which sizes every comparison PER
//! PAIR (§11.4.13, Table 11-21), while §12.5 sizes a normal case COLLECTIVELY, and
//! the reference tools split exactly there: sv2v (an `if` chain) is per pair,
//! verilator 5.052 is collective for the width and the case expression's sign but
//! extends each item by its OWN sign, and iverilog 13 rejects the construct. A
//! statement is accepted only where all three rules give one answer; every other
//! shape is ONE `VITA-E3009` naming the reason, and nothing of the statement is
//! lowered. Census: §4.5.582's grounding (c01–c40, m1–m9, f1/f2, s1–s3, v2, a1, the
//! 312-row sign/width matrix) and plan probes q1–q3, p5.

use super::*;
use hdl_ast::case_inside::{inside_label, InsideLabel};

/// Is `e` (parentheses stripped) an OPERATOR whose operands take the context width:
/// unary `+ - ~`, every binary operator except the comparisons and the logical
/// `&& ||`, and `?:`? The §12.5 context pass of `lower_case` re-lowers only these
/// (`case_operand_takes_ctx`); the case-inside guard asks the same question.
pub(crate) fn case_ctx_operator(e: &ast::Expr) -> bool {
    let mut n = e;
    while let ast::ExprKind::Paren { inner } = &n.kind {
        n = inner;
    }
    match &n.kind {
        ast::ExprKind::Binary { op, .. } => !matches!(
            op,
            ast::BinOp::Eq
                | ast::BinOp::Ne
                | ast::BinOp::CaseEq
                | ast::BinOp::CaseNe
                | ast::BinOp::WildEq
                | ast::BinOp::WildNe
                | ast::BinOp::InsideEq
                | ast::BinOp::Lt
                | ast::BinOp::Le
                | ast::BinOp::Gt
                | ast::BinOp::Ge
                | ast::BinOp::LogAnd
                | ast::BinOp::LogOr
        ),
        ast::ExprKind::Unary { op, .. } => {
            matches!(op, ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot)
        }
        ast::ExprKind::Ternary { .. } => true,
        _ => false,
    }
}

/// One lowered element of an item: a value `v`, or a range `[lo : hi]`.
#[derive(Clone, Copy)]
enum Elem<'a> {
    Value(&'a ast::Expr, u32),
    Range(&'a ast::Expr, u32, &'a ast::Expr, u32),
}

/// Why a statement is refused, and where to point.
struct Refusal {
    span: ast::Span,
    reason: &'static str,
}

const NOT_KNOWN: &str = "the case expression's or an item's width or signedness is not known here";

impl Elaborator<'_> {
    /// Lower `case (e) inside` (dispatched from `lower_case` before any §12.5 pass).
    pub(crate) fn lower_case_inside(
        &mut self,
        b: &mut ProcessBuilder,
        scrutinee: &ast::Expr,
        items: &[ast::CaseItem],
        case_span: ast::Span,
    ) {
        // A user `$` must never be read as a queue's last index here: `P` is never
        // lowered, a top-level `$` element is refused below, and a `q[$]` element
        // sets its own substitution inside `lower_dyn_index`.
        let saved = self.dollar_subst.take();
        self.lower_case_inside_body(b, scrutinee, items, case_span);
        self.dollar_subst = saved;
    }

    fn lower_case_inside_body(
        &mut self,
        b: &mut ProcessBuilder,
        scrutinee: &ast::Expr,
        items: &[ast::CaseItem],
        case_span: ast::Span,
    ) {
        // IEEE 1364 does not reserve `inside`, so in a design that also uses it as a
        // NAME, `case (x) inside[2:1]:` or `inside(3):` may be a plain `case` whose label
        // reads that name — iverilog `-g2005`, and every tool under `begin_keywords
        // "1364-2005"`, run it that way (review F1: 1/1/1 where the `inside` reading
        // gives 0/0/0). Which object the name binds is a scope question with several
        // binders (upward task search, loop variables, imports — review rounds 1–2), so
        // the decline is design-wide, keyed on the parser's token count, not on a scope
        // model: one refusal per statement, before anything is lowered.
        if let Some(at) = self.inside_name_use {
            let first = match self.span_resolver.map(|r| r.resolve(at.lo, at.hi)) {
                Some(l) => format!("{}:{}", l.file, l.line),
                None => format!("byte {}", at.lo),
            };
            let msg = format!(
                "`case … inside` is not supported in a design that also uses `inside` as a \
                 name (first at {first}); IEEE 1800-2017 reserves `inside`"
            );
            self.error_at(MsgCode::ElabUnsupported, case_span, &msg);
            return;
        }
        // Shape first, before anything is lowered: a label of any other shape is an
        // internal error, and a top-level `$` element would otherwise reach the
        // generic `$` lowering and say something about queues.
        for it in items {
            let ast::CaseItem::Match { labels, .. } = it else {
                continue;
            };
            for l in labels {
                let Some(el) = inside_label(l) else {
                    self.error_at(
                        MsgCode::ElabUnsupported,
                        l.span,
                        "internal: a `case … inside` label is not in the parser's shape",
                    );
                    return;
                };
                let dollar = match el {
                    InsideLabel::Value(v) => is_bare_dollar(v).then_some(v),
                    InsideLabel::Range(lo, hi) => [lo, hi].into_iter().find(|p| is_bare_dollar(p)),
                };
                if let Some(d) = dollar {
                    // c08a/b: sv2v has no parse for a `$` bound and verilator reads
                    // it unsigned where the §11.4.13 text reads the type's bound.
                    self.refuse_case_inside(Refusal {
                        span: d.span,
                        reason: "a `$` bound",
                    });
                    return;
                }
            }
        }

        let e0 = self.error_count;
        let eid = self.lower_expr(scrutinee);
        let mut tests: Vec<Vec<Elem>> = Vec::new();
        let mut arm_bodies: Vec<&ast::Stmt> = Vec::new();
        let mut default_body: Option<&ast::Stmt> = None;
        for it in items {
            match it {
                ast::CaseItem::Match { labels, body, .. } => {
                    let mut elems = Vec::with_capacity(labels.len());
                    for l in labels {
                        // Shape checked above.
                        match inside_label(l) {
                            Some(InsideLabel::Value(v)) => {
                                let vid = self.lower_expr(v);
                                elems.push(Elem::Value(v, vid));
                            }
                            Some(InsideLabel::Range(lo, hi)) => {
                                let lid = self.lower_expr(lo);
                                let hid = self.lower_expr(hi);
                                elems.push(Elem::Range(lo, lid, hi, hid));
                            }
                            None => return,
                        }
                    }
                    tests.push(elems);
                    arm_bodies.push(body);
                }
                // Last one wins, as for plain `case`.
                ast::CaseItem::Default { body, .. } => default_body = Some(body),
            }
        }
        // The first error already named its cause; a second line would only repeat it.
        if self.error_count > e0 {
            return;
        }
        let string_lane = self.ir_expr_is_string(eid);
        if let Some(r) = self.inside_refusal(scrutinee, eid, &tests, string_lane) {
            self.refuse_case_inside(r);
            return;
        }

        let merge = b.new_block();
        let arms: Vec<BlockId> = arm_bodies.iter().map(|_| b.new_block()).collect();
        let cap = self.hoist_case_scrutinee(b, eid, scrutinee.span, case_span);
        // Clause 6, second half: a case expression the capture could not take (a
        // class-method body reserves no case temp — f2: plain `case (g(n))` there
        // runs `g` once per tested label where both oracles run it once; a string)
        // is re-read by every test, so it must be repeatable.
        if cap == eid && !self.expr_is_repeatable(eid) {
            self.refuse_case_inside(Refusal {
                span: scrutinee.span,
                reason: "a call in an item, or a case expression that cannot be evaluated \
                         exactly once here",
            });
            return;
        }

        // Test cascade in source order: the first element answering `1'b1` takes its
        // arm. The engine takes a `Branch`'s then-side only on a known 1, so a test
        // that is x (an x/z case-expression bit against a 0/1 item bit) is no match,
        // as §12.5.4 says (c03, c12, c13g).
        for (elems, &arm) in tests.iter().zip(&arms) {
            for el in elems {
                let test = match *el {
                    Elem::Value(_, vid) if string_lane => {
                        self.case_cmp(cap, vid, ast::CaseKind::Case)
                    }
                    Elem::Value(_, vid) => match self.inside_value_cmp(cap, vid) {
                        Some(id) => id,
                        None => self.push_expr(ir::Expr::Binary {
                            op: ir::BinOp::Eq,
                            lhs: cap,
                            rhs: vid,
                        }),
                    },
                    // The `inside` operator's own range shape (`parse_inside`).
                    Elem::Range(_, lid, _, hid) => {
                        let ge = self.push_expr(ir::Expr::Binary {
                            op: ir::BinOp::Ge,
                            lhs: cap,
                            rhs: lid,
                        });
                        let le = self.push_expr(ir::Expr::Binary {
                            op: ir::BinOp::Le,
                            lhs: cap,
                            rhs: hid,
                        });
                        self.push_expr(ir::Expr::Binary {
                            op: ir::BinOp::LogAnd,
                            lhs: ge,
                            rhs: le,
                        })
                    }
                };
                let next = b.new_block();
                b.end_block_with(ir::Terminator::Branch {
                    cond: test,
                    then_bb: arm.raw(),
                    else_bb: next.raw(),
                });
                b.start_block(next);
            }
        }
        if let Some(body) = default_body {
            self.lower_stmt(b, body);
        }
        b.goto(merge);
        for (arm, body) in arms.into_iter().zip(arm_bodies) {
            b.start_block(arm);
            self.lower_stmt(b, body);
            b.goto(merge);
        }
        b.start_block(merge);
    }

    fn refuse_case_inside(&mut self, r: Refusal) {
        let msg = format!(
            "`case … inside` is not supported here: {} (IEEE 1800-2017 §12.5.4)",
            r.reason
        );
        self.error_at(MsgCode::ElabUnsupported, r.span, &msg);
    }

    /// The accept predicate (§4.5.582): `None` = every clause holds. Clauses are
    /// asked in order and the first that fails is the one reported.
    fn inside_refusal(
        &mut self,
        scrutinee: &ast::Expr,
        eid: u32,
        tests: &[Vec<Elem>],
        string_lane: bool,
    ) -> Option<Refusal> {
        // Every element operand, in source order.
        let parts: Vec<(&ast::Expr, u32, bool)> = tests
            .iter()
            .flatten()
            .flat_map(|el| match *el {
                Elem::Value(v, id) => vec![(v, id, true)],
                Elem::Range(lo, lid, hi, hid) => vec![(lo, lid, false), (hi, hid, false)],
            })
            .collect();
        let all = std::iter::once((scrutinee, eid, false)).chain(parts.iter().copied());
        let refuse = |span: ast::Span, reason: &'static str| Some(Refusal { span, reason });

        // 1. Domain and types. m7: verilator fails internally on a real case
        // expression and sv2v → iverilog rejects it, so no tool decides it.
        for (e, id, _) in all.clone() {
            if self.expr_is_real(id) {
                return refuse(e.span, "a `real` operand (no reference tool runs it)");
            }
            if matches!(self.ast_handle_kind(e), HKind::Handle | HKind::Null) {
                return refuse(e.span, "a class handle or `null` operand");
            }
        }
        const STRING_OUT: &str =
            "a `string` operand outside value items of a `string` case expression";
        if string_lane {
            // m7b (verilator): a string case expression against string values, the
            // compare plain `case` uses (`StrCmp == 0`). s1: a string RANGE has no
            // oracle (verilator fails internally in both spellings).
            for &(e, _, is_value) in &parts {
                let string_value = is_value
                    && (matches!(e.kind, ast::ExprKind::StrLit { .. })
                        || self.expr_is_string_ast(e));
                if !string_value {
                    return refuse(e.span, STRING_OUT);
                }
            }
        } else {
            // s2/s3 (P2): a string-returning call is string by its declaration but
            // not by its lowered node, and plain `case (sf(1))` is wrong for it today
            // (vita 0, verilator 3).
            for (e, id, _) in all.clone() {
                if self.expr_is_string_ast(e) || self.ir_expr_is_string(id) {
                    return refuse(e.span, STRING_OUT);
                }
            }
        }

        // 2. No fill literal: c39, verilator sizes `'1` collectively and sv2v
        // contradicts itself on it. (A top-level `$` was refused before lowering.)
        if !string_lane {
            for (e, _, _) in all.clone() {
                if expr_contains_fill(e) {
                    return refuse(e.span, "a fill literal (`'0`, `'1`, `'x`, `'z`)");
                }
            }
        }

        if !string_lane {
            // Width and signedness of every participant, from the engine's own table.
            let mut shape: Vec<(ast::Span, sim_ir::selfwidth::SelfWidth, bool, bool)> = Vec::new();
            for (e, id, _) in all.clone() {
                let Some(sw) = self.canonical_self_width(id) else {
                    return refuse(e.span, NOT_KNOWN);
                };
                let operator = self.inside_operator(e, id);
                let const_msb0 = !operator && self.const_msb_known_zero(id, sw.width);
                shape.push((e.span, sw, operator, const_msb0));
            }
            // The capture takes `ir_bits_of` / `expr_self_signed`; it must be the type
            // the engine evaluates. p5: `expr_self_signed` calls a call unsigned, so
            // `case (f(-1)) 4'sb1111:` with `function int f` is 0 on PRE where both
            // oracles print 1.
            let e_sw = shape[0].1;
            match self.ir_bits_of(eid) {
                Some(w) if w == e_sw.width => {}
                _ => return refuse(scrutinee.span, NOT_KNOWN),
            }
            if self.expr_self_signed(eid) != e_sw.signed {
                return refuse(
                    scrutinee.span,
                    "a call returning a signed type as the case expression (its \
                     signedness is not carried yet)",
                );
            }
            let w_all = shape.iter().map(|s| s.1.width).max().unwrap_or(1);

            // 3. Sign: every pair's sign must be the collective sign, and no signed
            // element may be one that verilator's own-sign extension widens
            // differently (c19a, c19c, matrix rows P≠C and V≠C; q1 B).
            let parts_shape = &shape[1..];
            let sign_ok = if e_sw.signed {
                parts_shape.iter().all(|s| s.1.signed)
            } else {
                parts_shape
                    .iter()
                    .all(|s| !s.1.signed || s.1.width == w_all || s.3)
            };
            if !sign_ok {
                let at = parts_shape
                    .iter()
                    .find(|s| {
                        if e_sw.signed {
                            !s.1.signed
                        } else {
                            s.1.signed && s.1.width != w_all && !s.3
                        }
                    })
                    .map_or(scrutinee.span, |s| s.0);
                return refuse(
                    at,
                    "items of mixed signedness against a signed case expression, or a \
                     narrow signed item in an unsigned comparison: the sizing rules of \
                     §12.5 and §11.4.13 give different answers here and the reference \
                     tools disagree",
                );
            }

            // 4. Width: an operator is evaluated at its pair's width, so that must be
            // the case-wide width (c19b: `case (a+b) inside 8'h00 … 16'h0100`).
            let width_bad = if shape[0].2 && e_sw.width != w_all {
                Some(scrutinee.span)
            } else {
                parts_shape
                    .iter()
                    .find(|s| s.2 && e_sw.width.max(s.1.width) != w_all)
                    .map(|s| s.0)
            };
            if let Some(at) = width_bad {
                return refuse(
                    at,
                    "an operator narrower than the widest operand of the case: the \
                     sizing rules give different answers here and the reference tools \
                     disagree",
                );
            }

            // 5. A value element whose x/z bits exist only at run time is compared
            // with `==`, where §11.4.13 needs `==?` (c30: vita 0, sv2v 1).
            for &(e, id, is_value) in &parts {
                let is_const = matches!(self.exprs.get(id as usize), Some(ir::Expr::Const { .. }));
                if is_value && !is_const && self.expr_may_be_unknown(id) {
                    return refuse(
                        e.span,
                        "an item whose x/z bits are known only at run time (a 4-state \
                         variable or a call): §11.4.13 compares it with `==?`, which \
                         needs a constant pattern here",
                    );
                }
            }
        }

        // 6. Effects: no tool orders item evaluation (m3: sv2v duplicates the calls,
        // verilator evaluates every item twice).
        for &(e, _, _) in &parts {
            if Self::assign_pattern_expr_has_call(e) {
                return refuse(
                    e.span,
                    "a call in an item, or a case expression that cannot be evaluated \
                     exactly once here",
                );
            }
        }
        None
    }

    /// An operand that takes its pair's width: by its source shape (seen through
    /// parentheses and `(min:typ:max)`, which lowers to `typ`) OR by its lowered node,
    /// whichever says so.
    fn inside_operator(&self, e: &ast::Expr, id: u32) -> bool {
        let mut n = e;
        loop {
            match &n.kind {
                ast::ExprKind::Paren { inner } => n = inner,
                ast::ExprKind::MinTypMax { typ, .. } => n = typ,
                _ => break,
            }
        }
        if case_ctx_operator(n) {
            return true;
        }
        match self.exprs.get(id as usize) {
            Some(ir::Expr::Binary { op, .. }) => match op {
                ir::BinOp::Add
                | ir::BinOp::Sub
                | ir::BinOp::Mul
                | ir::BinOp::Div
                | ir::BinOp::Mod
                | ir::BinOp::Pow
                | ir::BinOp::BitAnd
                | ir::BinOp::BitOr
                | ir::BinOp::BitXor
                | ir::BinOp::BitXnor
                | ir::BinOp::Shl
                | ir::BinOp::Shr
                | ir::BinOp::AShl
                | ir::BinOp::AShr => true,
                ir::BinOp::LogAnd
                | ir::BinOp::LogOr
                | ir::BinOp::Lt
                | ir::BinOp::Le
                | ir::BinOp::Gt
                | ir::BinOp::Ge
                | ir::BinOp::Eq
                | ir::BinOp::Ne
                | ir::BinOp::CaseEq
                | ir::BinOp::CaseNe
                | ir::BinOp::CasezEq
                | ir::BinOp::CasexEq => false,
            },
            Some(ir::Expr::Unary { op, .. }) => match op {
                ir::UnOp::Plus | ir::UnOp::Minus | ir::UnOp::BitNot => true,
                ir::UnOp::LogNot
                | ir::UnOp::RedAnd
                | ir::UnOp::RedNand
                | ir::UnOp::RedOr
                | ir::UnOp::RedNor
                | ir::UnOp::RedXor
                | ir::UnOp::RedXnor => false,
            },
            Some(ir::Expr::Ternary { .. }) => true,
            _ => false,
        }
    }

    /// A numeric constant of width `w` whose most significant bit is a known 0, so
    /// extending it by its own sign and by zero give the same value (q1 A: `[0:3]`,
    /// `32'sd7` against a 64-bit case expression, sv2v = verilator).
    fn const_msb_known_zero(&self, id: u32, w: u32) -> bool {
        let Some(ir::Expr::Const { val }) = self.exprs.get(id as usize) else {
            return false;
        };
        let Some(c) = self.consts.get(*val as usize) else {
            return false;
        };
        if c.repr != ir::ConstRepr::Numeric || c.width != w || w == 0 {
            return false;
        }
        let m = w - 1;
        let word = (m / 64) as usize;
        let bit = m % 64;
        let v = c.bits.val.get(word).copied().unwrap_or(0);
        let u = c.bits.unk.get(word).copied().unwrap_or(0);
        (v >> bit) & 1 == 0 && (u >> bit) & 1 == 0
    }
}

fn is_bare_dollar(e: &ast::Expr) -> bool {
    let mut n = e;
    while let ast::ExprKind::Paren { inner } = &n.kind {
        n = inner;
    }
    matches!(n.kind, ast::ExprKind::Dollar)
}
