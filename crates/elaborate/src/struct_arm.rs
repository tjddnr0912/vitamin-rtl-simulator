//! §3.a ⑤ⓚ: an assignment pattern `'{…}` as an arm of `?:` — or inside `( … )` —
//! whose target is a whole packed-struct variable or port.
//!
//! IEEE 1800-2017 §10.8 puts the second and third operands of a conditional operator,
//! and a parenthesized operand, in the same assignment-like context as the whole value,
//! so the pattern is typed by the TARGET: `exc_cause_o = irq ? C : '{irq_int: 1'b1, …};`
//! (`ibex_controller.sv:737`). The parser desugars a pattern only as a whole right-hand
//! side, against a table keyed by the variable's NAME; an arm reaches elaborate as the
//! pattern itself and was refused.
//!
//! Here the arm is rewritten to the concatenation the parser's whole-pattern desugar
//! builds (`build_struct_pattern_concat`: declaration order, `default:`, a fill at the
//! member's width, a 2-state member through `longint'`), against the members the parser
//! recorded AT the target's declaration (`NetVarDecl::pattern_members`), on the net the
//! lowering's own walk resolves the target to. Only where that is known to be the
//! declaration the name means:
//!
//! - a module-body scalar variable or an ANSI port (not `inout`) as wide as its members,
//!   recorded once under one key;
//! - a whole single-segment target that the scope walk resolves to that key;
//! - `=` and `<=` in a module's own process, and a continuous `assign` — never an inlined
//!   or framed subroutine body or a class method, and never `force` (the block-local
//!   scope-leak check walks no `force`, docs/PROBE_CATALOG.md);
//! - a design with no `inout` port anywhere (vita connects one from the parent only,
//!   W3056, so an arm's value crossing one is not the child's).
//!
//! Anything else keeps the refusal the pattern had.

use super::*;

impl Elaborator<'_> {
    /// Record the net just declared as `name` as a target whose packed-struct members
    /// the parser recorded at its declaration. A net recorded twice keeps no record.
    pub(crate) fn record_pattern_target(&mut self, name: &str, members: &[ast::PatternMember]) {
        if members.is_empty() {
            return;
        }
        let key = self.fq(name);
        let Some(&id) = self.symbols.get(&key) else {
            return;
        };
        let Some(net) = self.nets.get(id as usize) else {
            return;
        };
        let w: u64 = members.iter().map(|m| u64::from(m.width)).sum();
        if net.array_len != 1 || net.kind == ir::NetKind::Real || u64::from(net.width) != w {
            return;
        }
        if self.pattern_target_twice.contains(&id) {
            return;
        }
        if self.pattern_targets.remove(&id).is_some() {
            self.pattern_target_twice.insert(id);
            return;
        }
        self.pattern_targets.insert(id, (key, members.to_vec()));
    }

    /// For `=` and `<=`: when `rhs` has an assignment pattern as an arm of `?:` (or in
    /// parentheses), lower the target `lhs` FIRST (the rewrite needs the net it writes)
    /// and return it with the rewritten right-hand side, if any. Any other `rhs` returns
    /// `(None, None)` and the statement lowers in its usual order.
    pub(crate) fn struct_arm_target_first(
        &mut self,
        lhs: &ast::Lvalue,
        rhs: &ast::Expr,
    ) -> (Option<ir::Lvalue>, Option<ast::Expr>) {
        if self.pattern_targets.is_empty() || !has_arm_pattern(rhs) {
            return (None, None);
        }
        let lv = self.lower_lvalue(lhs);
        let rewritten = self.struct_arm_rhs(lhs, &lv, rhs);
        (Some(lv), rewritten)
    }

    /// The right-hand side with every assignment pattern reached through a `?:` arm or
    /// `( … )` rewritten to its member concatenation, for the target `lhs` lowered to
    /// `lv`. `None` when any condition of the module comment fails or any pattern does
    /// not fit the members; the caller then lowers `rhs` as before, which refuses it.
    pub(crate) fn struct_arm_rhs(
        &self,
        lhs: &ast::Lvalue,
        lv: &ir::Lvalue,
        rhs: &ast::Expr,
    ) -> Option<ast::Expr> {
        if self.pattern_targets.is_empty()
            || self.design_inout_port
            || !has_arm_pattern(rhs)
            || !self.struct_arm_context()
        {
            return None;
        }
        let ast::Lvalue::Ident(p) = lhs else {
            return None;
        };
        let [seg] = p.segments.as_slice() else {
            return None;
        };
        let [c] = lv.chunks.as_slice() else {
            return None;
        };
        if c.word.is_some() || c.offset.is_some() || c.width.is_some() {
            return None;
        }
        let (key, members) = self.pattern_targets.get(&c.net)?;
        if self.pkg_body_var(&seg.name).is_some()
            || self.subst_lookup(&seg.name).is_some()
            || self.out_subst_lookup(&seg.name).is_some()
        {
            return None;
        }
        let walked = self.walk_scopes_key(&seg.name, |k| self.symbols.contains_key(k))?;
        if walked != *key || self.symbols.get(&walked) != Some(&c.net) {
            return None;
        }
        rewrite_arms(rhs, members)
    }

    /// A module's own process or continuous assign: not an inlined or framed
    /// subroutine body, not a class method.
    fn struct_arm_context(&self) -> bool {
        !self.in_frame_body
            && self.inline_stack.is_empty()
            && self.cur_class_method.is_none()
            && self.subst.is_empty()
            && self.out_subst.is_empty()
            && !self
                .cur_prefix
                .split('.')
                .any(|s| s.starts_with("$func$") || s.starts_with("$itask$"))
    }
}

/// Does the design declare an `inout` port anywhere (a module, program or interface)?
pub(crate) fn design_has_inout_port(unit: &ast::SourceUnit) -> bool {
    unit.items.iter().any(|it| match it {
        ast::TopItem::Module(m) | ast::TopItem::Interface(m) => {
            let ansi = match &m.ports {
                ast::PortList::Ansi(ps) => ps.iter().any(|p| p.dir == ast::PortDir::Inout),
                _ => false,
            };
            ansi || m.body.iter().any(
                |b| matches!(b, ast::ModuleItem::PortDecl(pd) if pd.dir == ast::PortDir::Inout),
            )
        }
        _ => false,
    })
}

fn is_pattern(e: &ast::Expr) -> bool {
    matches!(
        e.kind,
        ast::ExprKind::AssignPattern(_) | ast::ExprKind::AssignPatternKeyed(_)
    )
}

/// Is there an assignment pattern at an arm position of `e` — an arm of a `?:` or the
/// operand of `( … )`, at any depth of those two? A whole pattern is not one.
pub(crate) fn has_arm_pattern(e: &ast::Expr) -> bool {
    let arm = |x: &ast::Expr| is_pattern(x) || has_arm_pattern(x);
    match &e.kind {
        ast::ExprKind::Ternary { then_e, else_e, .. } => arm(then_e) || arm(else_e),
        ast::ExprKind::Paren { inner } => arm(inner),
        _ => false,
    }
}

/// Does a KEYED pattern at an arm position of `e` read `name` — the values the rewrite
/// takes, which the shared read walker (`expr_reads_ident`) does not visit? The
/// block-local scope-leak check asks this of `=` and `<=`, so a block-local of the name
/// that an arm reads keeps its own net.
pub(crate) fn arm_reads_ident(e: &ast::Expr, name: &str) -> bool {
    let arm = |x: &ast::Expr| match &x.kind {
        ast::ExprKind::AssignPatternKeyed(kv) => {
            kv.iter().any(|(_, v)| da::expr_reads_ident(v, name))
        }
        _ => arm_reads_ident(x, name),
    };
    match &e.kind {
        ast::ExprKind::Ternary { then_e, else_e, .. } => arm(then_e) || arm(else_e),
        ast::ExprKind::Paren { inner } => arm(inner),
        _ => false,
    }
}

/// `e` with each pattern at an arm position rewritten to its member concatenation.
fn rewrite_arms(e: &ast::Expr, members: &[ast::PatternMember]) -> Option<ast::Expr> {
    let arm = |x: &ast::Expr| {
        if is_pattern(x) {
            pattern_concat(x, members)
        } else {
            rewrite_arms(x, members)
        }
    };
    let kind = match &e.kind {
        ast::ExprKind::Ternary {
            cond,
            then_e,
            else_e,
        } => ast::ExprKind::Ternary {
            cond: cond.clone(),
            then_e: Box::new(arm(then_e)?),
            else_e: Box::new(arm(else_e)?),
        },
        ast::ExprKind::Paren { inner } => ast::ExprKind::Paren {
            inner: Box::new(arm(inner)?),
        },
        _ => return Some(e.clone()),
    };
    Some(ast::Expr { kind, span: e.span })
}

/// The concatenation `{w0'(e0), …, wN'(eN)}` the pattern `pat` means for a struct of
/// `members` (the first is the MSB) — the parser's whole-pattern desugar for a flat
/// struct. `None` for a pattern that desugar refuses: a count or key that does not fit,
/// a duplicated key or `default:`, a call in `default:` (it would run once per member),
/// a pattern as a member's value, a 2-state member wider than 64 bits.
fn pattern_concat(pat: &ast::Expr, members: &[ast::PatternMember]) -> Option<ast::Expr> {
    let span = pat.span;
    let elems: Vec<ast::Expr> = match &pat.kind {
        ast::ExprKind::AssignPattern(v) => v.clone(),
        ast::ExprKind::AssignPatternKeyed(kv) => {
            let mut slots: Vec<Option<ast::Expr>> = vec![None; members.len()];
            let mut default: Option<&ast::Expr> = None;
            for (k, v) in kv {
                match k {
                    ast::AssignPatternKey::Default => {
                        if default.is_some() || Elaborator::assign_pattern_expr_has_call(v) {
                            return None;
                        }
                        default = Some(v);
                    }
                    ast::AssignPatternKey::Member(n) => {
                        let i = members.iter().position(|m| m.name == *n)?;
                        if slots[i].is_some() {
                            return None;
                        }
                        slots[i] = Some(v.clone());
                    }
                }
            }
            slots
                .into_iter()
                .map(|s| s.or_else(|| default.cloned()))
                .collect::<Option<Vec<_>>>()?
        }
        _ => return None,
    };
    if elems.len() != members.len() {
        return None;
    }
    let lit = |raw: String, kind: ast::IntLitKind| ast::Expr {
        kind: ast::ExprKind::IntLit { kind, raw },
        span,
    };
    let mut parts = Vec::with_capacity(elems.len());
    for (e, m) in elems.into_iter().zip(members) {
        if is_pattern(&e) || m.width == 0 {
            return None;
        }
        if let Some(bits) = fill_digit(&e, m.two_state) {
            let body: String = std::iter::repeat_n(bits, m.width as usize).collect();
            parts.push(lit(format!("{}'b{body}", m.width), ast::IntLitKind::Sized));
            continue;
        }
        if m.two_state && m.width > 64 {
            return None;
        }
        let inner = if m.two_state {
            ast::Expr {
                kind: ast::ExprKind::Cast {
                    target: ast::CastTarget::Prim(ast::CastPrim::Longint),
                    expr: Box::new(e),
                },
                span,
            }
        } else {
            e
        };
        parts.push(ast::Expr {
            kind: ast::ExprKind::Cast {
                target: ast::CastTarget::Size(Box::new(lit(
                    m.width.to_string(),
                    ast::IntLitKind::Decimal,
                ))),
                expr: Box::new(inner),
            },
            span,
        });
    }
    Some(ast::Expr {
        kind: ast::ExprKind::Concat { parts },
        span,
    })
}

/// The digit a fill literal (`'0` `'1` `'x` `'z`) repeats at a member's width; a 2-state
/// member stores an `x` or `z` fill as 0 (IEEE 1800-2017 §6.11.3).
fn fill_digit(e: &ast::Expr, two_state: bool) -> Option<char> {
    let ast::ExprKind::IntLit { raw, .. } = &e.kind else {
        return None;
    };
    match raw.trim() {
        "'0" => Some('0'),
        "'1" => Some('1'),
        "'x" | "'X" if !two_state => Some('x'),
        "'z" | "'Z" if !two_state => Some('z'),
        "'x" | "'X" | "'z" | "'Z" => Some('0'),
        _ => None,
    }
}
