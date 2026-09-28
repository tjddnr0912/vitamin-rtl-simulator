//! `'{default: v}` whose target is a whole PACKED variable or net (IEEE 1800-2017
//! §10.9.1): `logic [1:0] w = '{default: '0};`, or `assign c = '{default: RSP_DEFAULT};`
//! on `rsp_t [N-1:0] c`.
//!
//! Both oracles (verilator 5.052; sv2v 0.0.13 → iverilog 13) give every element of the
//! FIRST packed dimension the value `v`, as an assignment to that element: a bit of a
//! vector takes `v`'s low bit, an element `[3:0]` of `logic [1:0][3:0]` takes `v` sized to
//! four bits (sign-extended when `v` is signed), and a packed-struct element takes `v`
//! whole. The default never reaches an element's own dimensions or members. That value is
//! the replication `{N{W'(v)}}` of N elements of W bits, so the pattern is rewritten to it
//! and rides the replication and size-cast lowering: the IR is the IR of the spelled-out
//! replication.
//!
//! The element is known only when the first packed dimension is written in the target's
//! own declaration. A first dimension that a typedef supplies may be a packed struct's,
//! whose `'{default: v}` fills members instead, and elaborate cannot tell a struct from a
//! vector there, so such a target keeps the refusal.

use super::*;

impl Elaborator<'_> {
    /// Record the net just declared as `name` as a `'{default: v}` target whose first
    /// packed dimension has `n` elements and is written in its declaration.
    pub(crate) fn record_packed_default_target(&mut self, name: &str, n: u32) {
        if let Some(&id) = self.symbols.get(&self.fq(name)) {
            self.packed_default_nets.insert(id, n);
        }
    }

    /// For the statement forms that lower their right-hand side before their target: when
    /// `rhs` is `'{default: v}`, lower the target `lhs` FIRST (the rewrite needs its shape)
    /// and return it with the rewritten right-hand side, if any. Any other `rhs` returns
    /// `(None, None)` and the statement lowers in its usual order.
    pub(crate) fn packed_default_target_first(
        &mut self,
        lhs: &ast::Lvalue,
        rhs: &ast::Expr,
    ) -> (Option<ir::Lvalue>, Option<ast::Expr>) {
        if !is_default_pattern(rhs) {
            return (None, None);
        }
        let lv = self.lower_lvalue(lhs);
        let packed = self.packed_default_rhs(&lv, rhs);
        (Some(lv), packed)
    }

    /// The replication `{N{W'(v)}}` that `rhs = '{default: v}` means for the whole-net
    /// target `lv`. `None` when `rhs` is another expression or `lv` is not a recorded
    /// packed target; the caller then lowers `rhs` as before, which refuses the pattern.
    pub(crate) fn packed_default_rhs(&self, lv: &ir::Lvalue, rhs: &ast::Expr) -> Option<ast::Expr> {
        let ast::ExprKind::AssignPatternKeyed(keyed) = &rhs.kind else {
            return None;
        };
        let [(ast::AssignPatternKey::Default, v)] = keyed.as_slice() else {
            return None;
        };
        // `v` is evaluated once per element and §10.9.1 does not pin how many times, so a
        // call keeps the refusal, as in the unpacked-array expansion — also one that a
        // statement hoist has already moved into a temporary (`note_default_pattern_call`).
        // So does a `v` not known to be integral (`integral_operand`).
        if Self::assign_pattern_expr_has_call(v)
            || self
                .default_pattern_calls
                .contains(&(rhs.span.lo, rhs.span.hi))
            || !self.integral_operand(v)
        {
            return None;
        }
        let [c] = lv.chunks.as_slice() else {
            return None;
        };
        if c.word.is_some() || c.offset.is_some() || c.width.is_some() {
            return None;
        }
        let n = *self.packed_default_nets.get(&c.net)?;
        let net = self.nets.get(c.net as usize)?;
        if net.kind == ir::NetKind::Real || n == 0 || net.width % n != 0 {
            return None;
        }
        let span = rhs.span;
        let lit = |x: u32| ast::Expr {
            kind: ast::ExprKind::IntLit {
                kind: ast::IntLitKind::Decimal,
                raw: x.to_string(),
            },
            span,
        };
        let elem = ast::Expr {
            kind: ast::ExprKind::Cast {
                target: ast::CastTarget::Size(Box::new(lit(net.width / n))),
                expr: Box::new(v.clone()),
            },
            span,
        };
        Some(ast::Expr {
            kind: ast::ExprKind::Replicate {
                count: Box::new(lit(n)),
                value: vec![elem],
            },
            span,
        })
    }
}

impl Elaborator<'_> {
    /// Note a statement whose right-hand side is `'{default: v}` with a call in `v`,
    /// before any hoist rewrites it. The frame-call and system-call hoists move such a call
    /// into a temporary that `v` then reads, which would evaluate `v` once — a count the
    /// oracles split on (verilator evaluates it per element, sv2v → iverilog once). A
    /// rebuilt pattern keeps its span, so `packed_default_rhs` still refuses it.
    pub(crate) fn note_default_pattern_call(&mut self, s: &ast::Stmt) {
        let rhs = match s {
            ast::Stmt::Blocking { rhs, .. }
            | ast::Stmt::NonBlocking { rhs, .. }
            | ast::Stmt::Assign { rhs, .. }
            | ast::Stmt::Force { rhs, .. } => rhs,
            _ => return,
        };
        if let ast::ExprKind::AssignPatternKeyed(keyed) = &rhs.kind {
            if let [(ast::AssignPatternKey::Default, v)] = keyed.as_slice() {
                if Self::assign_pattern_expr_has_call(v) {
                    self.default_pattern_calls
                        .insert((rhs.span.lo, rhs.span.hi));
                }
            }
        }
    }

    /// Is `v` built only from values known to be integral — literals, numeric constants
    /// and plain packed variables, through operators, selects, concatenations and integral
    /// casts? `W'(v)` stands for an assignment of `v` to one element, and the size cast
    /// takes a real, string or class-handle operand differently from that assignment (it
    /// refuses a real one with a message naming a cast the source never wrote, converts a
    /// string variable both oracles refuse, and writes a handle's object id where the
    /// assignment is refused). A name is resolved by the lowering's own route
    /// (`bare_ident_route`); everything outside this list answers false and keeps the
    /// refusal: a real constant, a hierarchical reference, a container element, a cast to
    /// a real, class or parameter type.
    fn integral_operand(&self, v: &ast::Expr) -> bool {
        use ast::ExprKind as K;
        // The string lowering's own classifier: a `string` formal, a formal bound to a
        // string, a string array element.
        if self.expr_is_string_ast(v) {
            return false;
        }
        let ok = |e: &ast::Expr| self.integral_operand(e);
        match &v.kind {
            K::IntLit { .. } | K::StrLit { .. } => true,
            K::Ident(p) => match p.segments.as_slice() {
                [seg] => match self.bare_ident_route(&seg.name, v.span) {
                    // A string-valued parameter is its literal here, as in every
                    // assignment: the parser keeps no `string` keyword on a parameter.
                    BareIdentRoute::Param { .. }
                    | BareIdentRoute::Wide(_)
                    | BareIdentRoute::Str(_) => true,
                    // An inline formal: its actual is already lowered, and says.
                    BareIdentRoute::Subst(eid) => self.ir_plain_packed(eid),
                    BareIdentRoute::OutSubst(n) => self.plain_packed_net(n, false),
                    BareIdentRoute::Other => self
                        .lookup_net_scoped(&seg.name)
                        .is_some_and(|n| self.plain_packed_net(n, false)),
                    BareIdentRoute::Real(_) | BareIdentRoute::ArrayItem { .. } => false,
                },
                _ => self
                    .iface_member_net(p)
                    .is_some_and(|n| self.plain_packed_net(n, false)),
            },
            K::PkgScoped { pkg, name } => {
                let in_pkg = |m: Option<bool>| m.unwrap_or(false);
                let (p, n) = (&pkg.name, &name.name);
                if in_pkg(self.pkg_real_val.get(p).map(|m| m.contains_key(n))) {
                    false
                } else if in_pkg(self.pkg_str_raw.get(p).map(|m| m.contains_key(n)))
                    || in_pkg(self.pkg_wide_bits.get(p).map(|m| m.contains_key(n)))
                    || in_pkg(self.pkg_consts.get(p).map(|m| m.contains_key(n)))
                {
                    true
                } else {
                    self.pkg_vars
                        .get(p)
                        .and_then(|m| m.get(n))
                        .is_some_and(|&net| self.plain_packed_net(net, false))
                }
            }
            K::Paren { inner } => ok(inner),
            K::Unary { operand, .. } => ok(operand),
            K::Binary { lhs, rhs, .. } => ok(lhs) && ok(rhs),
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => ok(cond) && ok(then_e) && ok(else_e),
            K::BitSelect { base, index } => self.integral_select_base(base) && ok(index),
            K::PartSelect { base, msb, lsb } => {
                self.integral_select_base(base) && ok(msb) && ok(lsb)
            }
            K::IndexedPart {
                base,
                offset,
                width,
                ..
            } => self.integral_select_base(base) && ok(offset) && ok(width),
            K::Concat { parts } => parts.iter().all(ok),
            K::Replicate { count, value } => ok(count) && value.iter().all(ok),
            K::Cast { target, expr } => {
                let integral_target = match target {
                    ast::CastTarget::Size(w) => ok(w),
                    ast::CastTarget::Signing { .. } => true,
                    ast::CastTarget::Prim(prim) => !matches!(prim, ast::CastPrim::Real),
                    _ => false,
                };
                integral_target && ok(expr)
            }
            _ => false,
        }
    }

    /// The base of a select in [`Self::integral_operand`]: an element of a fixed unpacked
    /// array of such values is integral too, where the whole array has no value.
    fn integral_select_base(&self, base: &ast::Expr) -> bool {
        if let ast::ExprKind::Ident(p) = &base.kind {
            if let [seg] = p.segments.as_slice() {
                return matches!(
                    self.bare_ident_route(&seg.name, base.span),
                    BareIdentRoute::Other
                ) && self
                    .lookup_net_scoped(&seg.name)
                    .is_some_and(|n| self.plain_packed_net(n, true));
            }
        }
        self.integral_operand(base)
    }

    /// An already-lowered inline actual that is a plain packed value: not real, not a
    /// string, and not a read of a net [`Self::plain_packed_net`] refuses.
    fn ir_plain_packed(&self, eid: u32) -> bool {
        if self.expr_is_real(eid) || self.ir_expr_is_string(eid) {
            return false;
        }
        match self.exprs.get(eid as usize) {
            Some(ir::Expr::Signal { net, word: None }) => self.plain_packed_net(*net, false),
            Some(ir::Expr::Signal { net, word: Some(_) }) => self.plain_packed_net(*net, true),
            // A constant or an operator: a string or real one is caught above, and a
            // class-typed formal is a parse error, so no handle reaches an inline actual.
            Some(_) => true,
            None => false,
        }
    }

    /// A net holding a plain packed value: a wire, a variable or an `integer`, and not a
    /// class handle, an event, a string, a container handle or a real. `array` admits a
    /// fixed unpacked array of such elements.
    fn plain_packed_net(&self, n: u32, array: bool) -> bool {
        let Some(x) = self.nets.get(n as usize) else {
            return false;
        };
        matches!(
            x.kind,
            ir::NetKind::Wire | ir::NetKind::Reg | ir::NetKind::Logic | ir::NetKind::Integer
        ) && !self.net_class.contains_key(&n)
            && !self.event_nets.contains(&n)
            && !self.is_non_bit_addressable_target(n)
            && (array || !self.net_is_static_array(n))
    }
}

/// Is `rhs` exactly `'{default: v}`?
fn is_default_pattern(rhs: &ast::Expr) -> bool {
    matches!(
        &rhs.kind,
        ast::ExprKind::AssignPatternKeyed(k)
            if matches!(k.as_slice(), [(ast::AssignPatternKey::Default, _)])
    )
}

/// Is the first packed dimension `range` written in the declaration's own text, between
/// its start `lo` and its first name `first`? A typedef's dimension is the typedef's text.
/// A range the parser synthesizes shares one span with its bounds (a struct's, or a local
/// unpacked struct's `logic [W-1:0]`, whose span is the type name inside this window),
/// while a written one starts at its `[`, before its left bound.
pub(crate) fn first_dim_written(lo: u32, first: u32, range: Option<&ast::Range>) -> bool {
    range.is_some_and(|r| lo <= r.span.lo && r.span.lo < r.msb.span.lo && r.span.hi <= first)
}
