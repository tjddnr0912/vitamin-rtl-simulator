//! §11.6.1 width context for an INLINE (static) function body's assignments.
//!
//! The frame route gets this for free: `fname = e` / a body local writes a real
//! NET, and the engine evaluates an assignment's rhs at `max(lvalue_w, self_w)`,
//! so every context-determined operator inside the rhs folds at the declared
//! width. The INLINE fold has no net — the rhs becomes one substituted ExprId —
//! so the region folded at `max(operand self-widths)` and
//! [`Elaborator::resize_inline_assign`] then resized a value whose carry/product
//! bits were already gone: `function [31:0] fh(input [7:0] x); fh = fld * x;`
//! with `8'hFF` printed `00000001` where both oracles (iverilog 13, verilator
//! 5.052) print `0000fe01`, and the `automatic` twin beside it printed
//! `0000fe01` (ROADMAP §2 "Inline / frame binds", entry 1).
//!
//! The context is carried by the EXISTING §11.6.1 walk (`lower_expr_ctx`), which
//! already knows every Table 11-21 rule — a shift's right operand, a `**`
//! exponent, a comparison's operands, a concat's operands, a size cast and the
//! real/string/handle routes are all self-determined there, and stay so. This
//! module only opts that walk in for a fill-FREE rhs and widens the leaves it
//! reaches (`Elaborator::inline_ctx_ext`); `resize_inline_assign` still applies
//! the declared width/sign seal afterwards.

use super::*;

impl Elaborator<'_> {
    /// Lower an inline body's `fname = rhs` / `local = rhs` value in the §11.6.1
    /// context `ctx` (the declared return / local width). `ctx == 0` (unknown or
    /// implicit width) keeps the plain lowering, as before this slice.
    ///
    /// The EXTENSION SIGN is decided ONCE for the whole context-determined region
    /// by [`Elaborator::size_ctx_route`] — the same walk the size cast uses, for
    /// the same §11.8.1 reason: one unsigned leaf makes the whole expression
    /// unsigned and then EVERY leaf zero-extends, signed ones included
    /// (§4.5.212's measurement, re-measured here: `function [31:0] f; f = s8*b8;`
    /// with `s8 = -9`, `b8 = 8'hFF` is `0000f609` in both oracles — the
    /// sign-extended reading is `fffff709`). `None` from that route is an opaque
    /// leaf (a hierarchical read, a class field, a verbatim inline actual) whose
    /// width is a placeholder here; the rhs then keeps its pre-slice lowering.
    ///
    /// `target_is_bv` = the target's declared type is a BIT VECTOR. A
    /// `real`/`realtime` target is not a width context at all (§11.8.1: the
    /// integral rhs stays self-determined and its RESULT converts), so the opt-in
    /// must not fire for one — see `InlineScope::non_bv` for the measurement.
    pub(crate) fn lower_inline_assign_rhs(
        &mut self,
        e: &ast::Expr,
        ctx: u32,
        target_is_bv: bool,
    ) -> u32 {
        if ctx == 0 {
            return self.lower_expr(e);
        }
        if !target_is_bv || self.rhs_has_real_domain(e) {
            return self.lower_ctx_or_plain(e, ctx);
        }
        let Some((ext, _)) = self.size_ctx_route(e) else {
            return self.lower_ctx_or_plain(e, ctx);
        };
        let saved = std::mem::replace(&mut self.inline_ctx_ext, Some(ext));
        let id = self.lower_ctx_or_plain(e, ctx);
        self.inline_ctx_ext = saved;
        id
    }

    /// Does `e`'s CONTEXT-DETERMINED region carry a REAL-domain operand? Then the
    /// assignment is not a bit-width context for it at all (§11.8.1: the integral
    /// sub-expression is self-determined, computes at its own width, and the
    /// RESULT converts to real), and the opt-in must not fire:
    /// `function [31:0] f; real r; r = 2.5; f = a8 * b8 + r;` with `a8 = b8 =
    /// 8'hFF` is `4` in both oracles — `(255*255) mod 256 = 1`, then `1 + 2.5`,
    /// rounded — and `65028` if the region is widened first. Round-2 review
    /// measured 15 such cells.
    ///
    /// ⚠️ POLARITY. This is a GUARD, not a router: `true` costs only the opt-in
    /// (the rhs keeps its pre-slice lowering), `false` claims a bit-width domain.
    /// So every arm this walk cannot resolve answers `true`, and the match is
    /// `_`-free so a new `ExprKind` cannot join the `false` side by omission.
    /// That is the opposite polarity from `ast_has_real_call`, the propagation
    /// walk beside it (`_ => false`), which is why this is not that function.
    ///
    /// The DOMAIN rules are `sim_ir::realness::expr_is_real_node`'s, arm for arm
    /// (that is the shared rule both elaborate and the engine use): `+ - * / **`
    /// propagate, unary `+`/`-` propagate, a ternary takes its ARMS (not its
    /// condition), `$signed`/`$unsigned` are transparent, and the real-returning
    /// system functions are `real_math_arity` plus `$realtime`/`$itor`/
    /// `$bitstoreal`/`.atoreal()`. Everything whose RESULT is integral by
    /// construction — a comparison, a reduction, `!`, a shift, a bitwise op, a
    /// concat/replicate (§5.4.1), `$rtoi`, a size cast — is `false` even with a
    /// real underneath it, which is what keeps the slice's gain on
    /// `f = a8 * b8 + (r > 1.0)` (both oracles `65025`).
    ///
    /// Leaf census — every producer of a real-domain NAME, with the resolver the
    /// lowering itself uses (`bare_ident_route`, in its order): an inline formal
    /// or body local bound by substitution (`Subst` ⇒ the actual's own
    /// `expr_is_real`, which is how `real r; r = 2.5;` is seen), a `real`
    /// PARAMETER (`Real`), a real net/var/module-level variable (the `Other`
    /// tail ⇒ `lookup_net_scoped` ⇒ `NetKind::Real`), a real DynArray element
    /// (`real_elem_dyn_nets`, reached through the same net test), a real-returning
    /// user function (`lookup_func`), a `real'(…)` cast, and a real literal. A
    /// hierarchical / package-scoped / class-member name is a placeholder here,
    /// so it answers `true` rather than guessing.
    pub(crate) fn rhs_has_real_domain(&self, e: &ast::Expr) -> bool {
        use ast::ExprKind as K;
        let any = |xs: &[ast::Expr]| xs.iter().any(|x| self.rhs_has_real_domain(x));
        match &e.kind {
            K::RealLit { .. } => true,
            K::IntLit { .. } | K::StrLit { .. } | K::Null | K::Dollar => false,
            K::Paren { inner } => self.rhs_has_real_domain(inner),
            K::MinTypMax { typ, .. } => self.rhs_has_real_domain(typ),
            K::Unary { op, operand } => {
                // Only `+`/`-` are real-preserving; `~`, `!` and the reductions
                // have an integral result (and a real operand there is E3009).
                matches!(op, ast::UnOp::Plus | ast::UnOp::Minus)
                    && self.rhs_has_real_domain(operand)
            }
            K::Binary { op, lhs, rhs } => {
                matches!(
                    op,
                    ast::BinOp::Add
                        | ast::BinOp::Sub
                        | ast::BinOp::Mul
                        | ast::BinOp::Div
                        | ast::BinOp::Pow
                ) && (self.rhs_has_real_domain(lhs) || self.rhs_has_real_domain(rhs))
            }
            K::Ternary { then_e, else_e, .. } => {
                self.rhs_has_real_domain(then_e) || self.rhs_has_real_domain(else_e)
            }
            // A select's RESULT is integral (§11.8.1) — except that the same `[i]`
            // spelling is an unpacked-array ELEMENT read, which carries the
            // element's own domain, so the base is walked rather than assumed.
            K::BitSelect { base, .. } | K::PartSelect { base, .. } => {
                self.rhs_has_real_domain(base)
            }
            K::IndexedPart { base, .. } => self.rhs_has_real_domain(base),
            // §5.4.1: a concat / replicate is an unsigned bit vector.
            K::Concat { .. } | K::Replicate { .. } => false,
            K::SysCall { name, args } => match systask::map_sysfunc(&name.name) {
                // transparent to the argument's domain (§4.5.317)
                Some(ir::SysFuncId::Signed | ir::SysFuncId::Unsigned) => any(args),
                Some(w) => {
                    ir::realness::real_math_arity(w).is_some()
                        || matches!(
                            w,
                            ir::SysFuncId::Realtime
                                | ir::SysFuncId::Itor
                                | ir::SysFuncId::BitsToReal
                                | ir::SysFuncId::StrAtoreal
                                | ir::SysFuncId::ArrSum
                                | ir::SysFuncId::ArrProduct
                        )
                }
                // an unmapped `$name` is not classified here
                None => true,
            },
            K::Call { name, args } => {
                let _ = args; // a call's ARGS are their own context (§13.5.3)
                name.segments.len() != 1
                    || self.lookup_func(&name.segments[0].name).is_none_or(|f| {
                        matches!(f.ret_type, ast::ParamType::Real | ast::ParamType::Realtime)
                    })
            }
            K::Cast { target, expr } => match target {
                ast::CastTarget::Prim(ast::CastPrim::Real) => true,
                ast::CastTarget::Prim(_) | ast::CastTarget::Size(_) => false,
                ast::CastTarget::Signing { .. } => self.rhs_has_real_domain(expr),
                // a typedef / `parameter type` name can resolve to `real`
                ast::CastTarget::Named(_) => true,
                // the signing half of a `parameter type` T'(e): T can be `real`
                ast::CastTarget::SigningParam { .. } => true,
            },
            K::Ident(p) => {
                if p.segments.len() != 1 {
                    return true; // hierarchical / class-member: a placeholder here
                }
                let name = &p.segments[0].name;
                match self.bare_ident_route(name, e.span) {
                    BareIdentRoute::Subst(eid) => self.expr_is_real(eid),
                    BareIdentRoute::Real(_) => true,
                    BareIdentRoute::ArrayItem { .. }
                    | BareIdentRoute::Wide(_)
                    | BareIdentRoute::Param { .. } => false,
                    BareIdentRoute::OutSubst(net) => self.net_is_real(net),
                    // a `string` parameter is not a real, but it is not a bit
                    // vector either — declining costs only the opt-in.
                    BareIdentRoute::Str(_) => true,
                    BareIdentRoute::Other => self
                        .lookup_net_scoped(name)
                        .is_none_or(|net| self.net_is_real(net)),
                }
            }
            // Not resolved by this walk ⇒ no bit-width claim.
            K::PkgScoped { .. }
            | K::MethodCall { .. }
            | K::RandomizeWith(_)
            | K::ArrayMethodWith(_)
            | K::New { .. }
            | K::ClassNew { .. }
            | K::TimeLit { .. }
            | K::NamedArg { .. }
            | K::Dist { .. }
            | K::AssignPattern(_)
            | K::AssignPatternKeyed(_)
            | K::Error => true,
        }
    }

    /// Is `net` a `real` (or a DynArray whose ELEMENTS are real)? The same two
    /// tests `sim_ir::realness`'s `Signal` arm makes.
    fn net_is_real(&self, net: u32) -> bool {
        self.nets
            .get(net as usize)
            .is_some_and(|n| matches!(n.kind, ir::NetKind::Real))
            || self.real_elem_dyn_nets.contains(&net)
    }

    /// `lower_expr` for a SELF-determined position inside the context walk: the
    /// opt-in is cleared for the whole sub-lowering, so nothing under a shift
    /// amount, an exponent, a ternary condition or a nested call's formal bind
    /// takes this rhs's context.
    pub(crate) fn lower_self_det(&mut self, e: &ast::Expr) -> u32 {
        let saved = self.inline_ctx_ext.take();
        let id = self.lower_expr(e);
        self.inline_ctx_ext = saved;
        id
    }

    /// [`Self::lower_self_det`] for the `lower_expr_ungated` entry (the node that
    /// `lower_expr_ctx` hands back for ordinary lowering).
    pub(crate) fn lower_ungated_self_det(&mut self, e: &ast::Expr) -> u32 {
        let saved = self.inline_ctx_ext.take();
        let id = self.lower_expr_ungated(e);
        self.inline_ctx_ext = saved;
        id
    }

    /// The `lower_expr_ctx` LEAF arm: lower `e` with the opt-in cleared (a leaf's
    /// own operands are self-determined), then widen the RESULT to the context —
    /// the §11.6.1 conversion, applied at the context boundary and not inside the
    /// leaf.
    pub(crate) fn lower_leaf_in_ctx(&mut self, e: &ast::Expr, ctx: u32) -> u32 {
        let on = self.inline_ctx_ext.take();
        let id = self.lower_expr_ungated(e);
        self.inline_ctx_ext = on;
        match on {
            Some(ext) => self.widen_inline_leaf(id, ctx, ext),
            None => id,
        }
    }

    /// Extend `e` to `ctx` bits by the REGION's sign `ext` (not the leaf's own —
    /// see [`Self::lower_inline_assign_rhs`]). A signed region keeps its leaves
    /// signed through the widening: `extend_to` builds an unsigned `Concat`, so
    /// the `$signed` stamp is what makes the operator's own sign rule see a
    /// signed operand.
    ///
    /// Four stand-downs, each keeping the pre-slice value rather than guessing:
    ///  * a real or string value has no bit width to extend (§6.12 / §6.16);
    ///  * an UNTRUSTED width is a fabricated one (a class field reads through a
    ///    32-bit handle) — `trusted_self_width` is the one spelling of that guard,
    ///    and `resize_inline_assign` stands down on the same answer;
    ///  * `w >= ctx` is not a widening — §11.6.1 only grows an operand, and a
    ///    narrower context never truncates one;
    ///  * a SIGNED extension's fill is the operand's own MSB, i.e. a SECOND
    ///    mention of `e`, and the engine walks the DAG as a tree — so an operand
    ///    that cannot be drawn twice (`$random`, a frame call: `expr_is_repeatable`)
    ///    is left alone. The UNSIGNED fill is a constant, so it names `e` once and
    ///    needs no such gate.
    pub(crate) fn widen_inline_leaf(&mut self, e: u32, ctx: u32, ext: bool) -> u32 {
        if ctx == 0 || self.expr_is_real(e) || self.ir_expr_is_string(e) {
            return e;
        }
        let Some(w) = self.trusted_self_width(e) else {
            return e;
        };
        if w >= ctx {
            return e;
        }
        if !ext {
            return self.extend_to(e, w, ctx, false);
        }
        if !self.expr_is_repeatable(e) {
            return e;
        }
        let ext_id = self.extend_to(e, w, ctx, true);
        self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::Signed,
            args: vec![ext_id],
        })
    }
}
