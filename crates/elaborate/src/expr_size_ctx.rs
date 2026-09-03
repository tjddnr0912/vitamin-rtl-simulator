//! size-cast CONTEXT lowering — `N'(expr)` over a context-determined operation
//! (§4.5.212 width, §4.5.316 `max(self, N)` + sign propagation, §4.5.317 the real
//! refusal funnel). Split out of `expr_cast.rs` at the 1000-line policy; the cast
//! ARMS themselves stay there and enter here through `lower_size_ctx_entry`.

use super::*;

/// The ONE spelling of the size-cast real refusal. It lived at two sites before
/// §4.5.317 and the funnel would have made it three.
pub(crate) const REAL_SIZE_CAST_MSG: &str =
    "size cast is not defined on a real operand (use int'/longint')";

/// See [`Elaborator::select_chain`].
enum SelChain {
    /// a bit of `net` (or of a constant: `None`) — 1-bit unsigned
    Bit { net: Option<u32> },
    /// an unpacked-array element; `None` where the element's sign / width is
    /// not visible here (a dynamic handle)
    Elem {
        signed: Option<bool>,
        width: Option<u32>,
    },
    /// fewer selects than unpacked dimensions: not a value
    NotAValue,
    /// not a select chain down to a name this walk resolves
    Unknown,
}

impl Elaborator<'_> {
    /// §4.5.212: is `e` a CONTEXT-DETERMINED operation whose result width the size
    /// cast `N'(e)` must drive down into (arith/bitwise/shift/`**`, unary `+`/`-`/`~`,
    /// ternary)? Such an operation computes at its operands' self-width by default, so
    /// a cast to a WIDER N would silently lose the carry (`8'(a*b)` = 13 vs 45). A bare
    /// leaf, select, concat, or comparison is SELF-determined — the cast just resizes
    /// its already-correct value, so the existing fill-only path stays byte-identical.
    pub(crate) fn is_size_ctx_operation(e: &ast::Expr) -> bool {
        match &e.kind {
            ast::ExprKind::Paren { inner } => Self::is_size_ctx_operation(inner),
            ast::ExprKind::Binary { op, .. } => matches!(
                op,
                ast::BinOp::Add
                    | ast::BinOp::Sub
                    | ast::BinOp::Mul
                    | ast::BinOp::Div
                    | ast::BinOp::Mod
                    | ast::BinOp::BitAnd
                    | ast::BinOp::BitOr
                    | ast::BinOp::BitXor
                    | ast::BinOp::BitXnor
                    | ast::BinOp::Shl
                    | ast::BinOp::Shr
                    | ast::BinOp::AShl
                    | ast::BinOp::AShr
                    | ast::BinOp::Pow
            ),
            ast::ExprKind::Unary { op, .. } => {
                matches!(op, ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot)
            }
            ast::ExprKind::Ternary { .. } => true,
            _ => false,
        }
    }

    /// §4.5.212: the OVERALL self-signedness of a size-cast operand's tree — signed iff
    /// EVERY context-determined leaf is signed (§11.8.1: any unsigned operand makes the
    /// whole expression unsigned, and ALL leaves are then zero-extended — verified
    /// against iverilog: `8'((signed*signed)+unsigned)` zero-extends the signed pair).
    /// A single sign therefore governs every leaf's extension. `None` ⇒ a leaf whose
    /// sign can't be resolved here (a call / hierarchical ref / string or real
    /// constant) → the caller keeps the fill-only behavior. ⚠️ `None` is load-bearing
    /// in BOTH directions: answering it where `Some` was possible drops the context
    /// width (the fill-only path computes at self width — 170 element cells and 114
    /// constant cells regressed/were wrong that way), and a WRONG `Some` extends a
    /// leaf against the lowering (§4.5.393's blocking shape). Every `Some` here must
    /// come from the resolver the lowering itself uses. Mirrors `expr_self_signed`'s
    /// operator rules.
    /// §4.5.212: the OVERALL self-signedness of a size-cast operand's tree.
    ///
    /// An OPAQUE leaf — a hierarchical or class-member read, a hierarchical call,
    /// an inline formal bound to one — is a PLACEHOLDER at this point (resolved
    /// only after the instance tree exists) with no width for `lower_size_leaf`
    /// to resize by, so a cast over one is right on neither path; when the
    /// operand holds one ANYWHERE, every leaf rule falls back to the pre-slice
    /// classifier verbatim and the cast keeps exactly the answer it had.
    /// Measured in both directions: resolving a constant sibling routed
    /// `16'(PS16 * u.a1[2])` over the widthless leaf (`xxxx` for the oracles'
    /// `4d20`), and declining the leaf sent `8'(-u.v[3])` to the fill-only path
    /// (`01` for `ff`).
    /// `consts` = the operand holds NO opaque leaf, so the constant-resolving
    /// rules apply; `false` selects the pre-slice classifier verbatim.
    /// [`Self::size_ctx_route`] computes it once for both walks.
    fn ctx_signed_impl(&self, e: &ast::Expr, consts: bool) -> Option<bool> {
        match &e.kind {
            ast::ExprKind::Paren { inner } => self.ctx_signed_impl(inner, consts),
            ast::ExprKind::Binary { op, lhs, rhs } => match op {
                ast::BinOp::Add
                | ast::BinOp::Sub
                | ast::BinOp::Mul
                | ast::BinOp::Div
                | ast::BinOp::Mod
                | ast::BinOp::BitAnd
                | ast::BinOp::BitOr
                | ast::BinOp::BitXor
                | ast::BinOp::BitXnor => {
                    Some(self.ctx_signed_impl(lhs, consts)? && self.ctx_signed_impl(rhs, consts)?)
                }
                // power / shifts: sign follows the LEFT (base) operand only.
                ast::BinOp::Pow
                | ast::BinOp::Shl
                | ast::BinOp::Shr
                | ast::BinOp::AShl
                | ast::BinOp::AShr => self.ctx_signed_impl(lhs, consts),
                // comparison / logical / wildcard: a 1-bit UNSIGNED result.
                _ => Some(false),
            },
            ast::ExprKind::Unary { op, operand } => match op {
                ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot => {
                    self.ctx_signed_impl(operand, consts)
                }
                _ => Some(false), // reductions / logical-not: 1-bit unsigned
            },
            ast::ExprKind::Ternary { then_e, else_e, .. } => Some(
                self.ctx_signed_impl(then_e, consts)? && self.ctx_signed_impl(else_e, consts)?,
            ),
            // concat / replicate are ALWAYS unsigned (§5.4.1).
            ast::ExprKind::Concat { .. } | ast::ExprKind::Replicate { .. } => Some(false),
            // A select of a PACKED vector is unsigned too (§5.4.1) — but the very
            // same AST spelling is an UNPACKED ARRAY ELEMENT read, and an element
            // is not a select at all: it carries the ELEMENT's declared sign.
            // Calling those unsigned built the whole cast operand unsigned, so
            // `40'(sm[0]*1)` on `logic signed [7:0] sm[0:3]` printed `…00fd`
            // where iverilog prints `…fffd`. It stayed invisible until the cast
            // started sealing, because an unsealed node let the destination
            // context re-run the operation wider and land back on the oracle by
            // accident (`1'(-sm[0])`, and `40'(sm[1]*1)` on an uninitialised
            // element — the t=0 state of ordinary RTL).
            //
            // ⚠️ The answer is the element's SIGN, not `None`. `None` means "I
            // cannot resolve this leaf", which routes the whole cast to the
            // fill-only path — and that path drops §4.5.212's context width, so
            // `7'(sm[0] + 4'h3)` fell from the oracles' `10` to `00`: **170 cells
            // regressed** in a 1,560-cell element matrix before this was measured.
            // Most element operands sit next to an unsigned sibling, where
            // §11.8.1 makes the whole expression unsigned anyway and the old
            // blanket `false` was accidentally right; only an all-signed
            // expression discriminates.
            // A packed part-select is unsigned whatever it selects from (§5.4.1)…
            ast::ExprKind::PartSelect { .. } | ast::ExprKind::IndexedPart { .. } => Some(false),
            // …a chain of `[i]` selects is unsigned when it ends at a BIT (of a
            // vector, of a constant, of an element — a bit-select is 1-bit
            // unsigned whatever its base, §11.8.1, and the fill-only path would
            // evaluate that 1-bit leaf at 1 bit: `8'(-a1[0][3])` `ff` → `01`), and
            // carries the ELEMENT's sign when it ends at one (a 1-D or N-D static
            // array's element, a frame array's element); an element whose sign
            // this walk cannot see (a dynamic / queue / assoc handle) is `None`,
            // not `false` — `64'(PS16 / g2[0][1])` extended a signed element as
            // unsigned once its constant sibling became resolvable. A base the
            // walk cannot resolve keeps the pre-slice `false`.
            // …and a chain the walk cannot resolve at all (a hierarchical or
            // class-member base — a placeholder here, sized only after the
            // instance tree exists) declines: answering `false` routed the cast
            // over a widthless leaf whenever its constant sibling resolved
            // (`16'(PS16 * u.a1[2])` printed `xxxx` for the oracles' `4d20`).
            ast::ExprKind::BitSelect { .. } if !consts => Some(self.pre_slice_elem_signed(e)),
            ast::ExprKind::BitSelect { .. } => match self.select_chain(e) {
                SelChain::Bit { .. } => Some(false),
                SelChain::Elem { signed, .. } => signed,
                SelChain::NotAValue | SelChain::Unknown => None,
            },
            ast::ExprKind::IntLit { kind, raw } => {
                if literal::is_fill_literal(raw, *kind) {
                    return Some(false);
                }
                literal::parse_int_literal(raw, *kind).map(|c| c.signed)
            }
            // A bare name binds where `lower_expr` binds it — `bare_ident_route`
            // is the lowering's own decision, in its order (iterator → inline
            // formal → task out-formal → string/wide/real → numeric constant →
            // everything else). The sign of each route is the sign of the node
            // that route builds: a substituted actual's own (`expr_self_signed`,
            // the rule `lower_size_leaf` stamps by — but only for an actual
            // `resize_inline_assign` built; one handed over VERBATIM has a mirror
            // the engine does not honour, see `verbatim_actuals`), the wide const's, the
            // constant's `param_const_signed` — the rule `const_param_expr_w`
            // builds with. A string or real read is not a bit-vector ⇒ `None`
            // (the real refusal keeps firing). `Other` is the tail the lowering
            // resolves through `resolve_net`: answered as before, by the net.
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                let name = &p.segments[0].name;
                if !consts {
                    let net = self.lookup_net_scoped(name)?;
                    return Some(self.nets.get(net as usize)?.signed);
                }
                match self.bare_ident_route(name, e.span) {
                    BareIdentRoute::ArrayItem { signed, .. } => Some(signed),
                    BareIdentRoute::Subst(eid) => {
                        (!self.verbatim_actuals.contains(&eid)).then(|| self.expr_self_signed(eid))
                    }
                    BareIdentRoute::OutSubst(net) => {
                        if self.net_is_static_array(net) {
                            return None;
                        }
                        Some(self.nets.get(net as usize)?.signed)
                    }
                    BareIdentRoute::Str(_) | BareIdentRoute::Real(_) => None,
                    BareIdentRoute::Wide(cv) => Some(cv.signed),
                    // a guessed type (an overridden untyped parameter, §2 row 25, and
                    // what derives from it) is not a sign to extend by — pre-slice route
                    BareIdentRoute::Param { guessed: true, .. } => None,
                    BareIdentRoute::Param { v, meta, .. } => {
                        Some(Self::param_const_signed(v, meta))
                    }
                    BareIdentRoute::Other => {
                        let net = self.lookup_net_scoped(name)?;
                        Some(self.nets.get(net as usize)?.signed)
                    }
                }
            }
            ast::ExprKind::PkgScoped { pkg, name } if consts => {
                self.pkg_const_read_signed(&pkg.name, &name.name)
            }
            // calls / sysfuncs / hierarchical refs / patterns → indeterminate here.
            // A call is its own slice: `expr_self_signed`'s `_ => false` has 21
            // callers, so answering it here would widen a 2-site blast radius to 21.
            _ => None,
        }
    }

    /// `pkg::NAME` twin of the `Ident` arm of [`Self::ast_ctx_signed`], mirroring the
    /// `PkgScoped` arm of `lower_expr` in ITS order (real → string → wide →
    /// numeric → package variable). A whole unpacked package array is an error
    /// there, so it is `None` here.
    fn pkg_const_read_signed(&self, pkg: &str, name: &str) -> Option<bool> {
        if self
            .pkg_real_val
            .get(pkg)
            .is_some_and(|m| m.contains_key(name))
            || self
                .pkg_str_raw
                .get(pkg)
                .is_some_and(|m| m.contains_key(name))
        {
            return None;
        }
        if let Some(cv) = self.pkg_wide_bits.get(pkg).and_then(|m| m.get(name)) {
            return Some(cv.signed);
        }
        if let Some(&v) = self.pkg_consts.get(pkg).and_then(|m| m.get(name)) {
            let meta = self
                .pkg_const_meta
                .get(pkg)
                .and_then(|m| m.get(name))
                .copied();
            return Some(Self::param_const_signed(v, meta));
        }
        let net = *self.pkg_vars.get(pkg)?.get(name)?;
        if self.net_is_static_array(net) {
            return None;
        }
        Some(self.nets.get(net as usize)?.signed)
    }

    /// What a chain of `[i]` selects down to a bare name denotes — see the
    /// `BitSelect` arms of [`Self::ast_ctx_signed`] and
    /// [`Self::size_ctx_self_width`]. `k` selects on a name with `d` unpacked
    /// dimensions: `k > d` is a BIT (of the element, or of a plain vector / a
    /// constant: `d == 0`), `k == d` an ELEMENT, `k < d` a sub-array (not a
    /// value). `d` is `array_dims`' count for a static array (absent = 1), 1 for
    /// a dynamic / queue / assoc handle and for a frame-local / formal array (an
    /// md-packed slot, `frame_arr_formal_meta`), 0 for anything else. The base
    /// is a bare single-segment name or `pkg::name`; any other base (a
    /// hierarchical element, a call) is `Unknown`.
    fn select_chain(&self, e: &ast::Expr) -> SelChain {
        let mut k = 0u32;
        let mut b = e;
        loop {
            match &b.kind {
                ast::ExprKind::Paren { inner } => b = inner,
                ast::ExprKind::BitSelect { base, .. } => {
                    k += 1;
                    b = base;
                }
                _ => break,
            }
        }
        if k == 0 {
            return SelChain::Unknown;
        }
        let net = match &b.kind {
            // …resolved by the SAME route the rest of this file and the lowering
            // take. Reading `lookup_net_scoped` + `lookup_scoped` directly (the
            // net table and the i64 constant domain) missed every name those two
            // maps do not carry — a parameter wider than i64 (`wide_param_bits`)
            // and an inline-substituted formal — so a BIT of one declined and the
            // fill-only path evaluated a 1-bit leaf at 1 bit (round-6 review: 108
            // cells where PRE routed and was right).
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                match self.bare_ident_route(&p.segments[0].name, b.span) {
                    BareIdentRoute::OutSubst(net) => net,
                    BareIdentRoute::Other => match self.lookup_net_scoped(&p.segments[0].name) {
                        Some(net) => net,
                        None => return SelChain::Unknown,
                    },
                    // a constant, a substituted actual, a `with`-clause item: a
                    // packed VALUE, so a `[i]` of it is a 1-bit unsigned bit
                    _ => return SelChain::Bit { net: None },
                }
            }
            // …and the package twin, in the SAME order the `PkgScoped` arms of
            // `ast_ctx_signed` / `size_ctx_self_width` use: a wide package
            // parameter lives in its own side map, and reading `pkg_consts`
            // alone made a bit of one decline (round-6: `2'(pk::WP[95] + …)`).
            ast::ExprKind::PkgScoped { pkg, name } => {
                let has_const = self
                    .pkg_consts
                    .get(&pkg.name)
                    .is_some_and(|m| m.contains_key(&name.name))
                    || self
                        .pkg_wide_bits
                        .get(&pkg.name)
                        .is_some_and(|m| m.contains_key(&name.name));
                match self.pkg_vars.get(&pkg.name).and_then(|m| m.get(&name.name)) {
                    Some(&net) => net,
                    None if has_const => return SelChain::Bit { net: None },
                    None => return SelChain::Unknown,
                }
            }
            // a packed VALUE (a part-select, a concat, a call, an operator, a
            // cast, a literal): its bit is a bit — 1-bit unsigned, as before
            ast::ExprKind::PartSelect { .. }
            | ast::ExprKind::IndexedPart { .. }
            | ast::ExprKind::Concat { .. }
            | ast::ExprKind::Replicate { .. }
            | ast::ExprKind::Cast { .. }
            | ast::ExprKind::SysCall { .. }
            | ast::ExprKind::Call { .. }
            | ast::ExprKind::Binary { .. }
            | ast::ExprKind::Unary { .. }
            | ast::ExprKind::Ternary { .. }
            | ast::ExprKind::IntLit { .. } => return SelChain::Bit { net: None },
            // a hierarchical or class-member base is a PLACEHOLDER at this point
            // (resolved after the instance tree exists): neither its element-ness
            // nor its width is knowable here
            _ => return SelChain::Unknown,
        };
        if let Some(af) = self.frame_arr_formal_meta.get(&net) {
            // N-D: `lower_packed_read` builds the element at exactly `dims.len()`
            // selects (and re-stamps its sign from `elem_signed` there).
            let d = af.dims.len().max(1) as u32;
            return match k.cmp(&d) {
                std::cmp::Ordering::Less => SelChain::NotAValue,
                std::cmp::Ordering::Equal => SelChain::Elem {
                    signed: Some(af.elem_signed),
                    width: Some(af.elem_w),
                },
                std::cmp::Ordering::Greater => SelChain::Bit { net: None },
            };
        }
        if self.is_dyn_handle_net(net) {
            return if k == 1 {
                SelChain::Elem {
                    signed: None,
                    width: None,
                }
            } else {
                SelChain::Bit { net: None }
            };
        }
        if self.net_is_static_array(net) {
            let d = self.array_dims.get(&net).map_or(1, |v| v.len() as u32);
            return match k.cmp(&d) {
                std::cmp::Ordering::Less => SelChain::NotAValue,
                std::cmp::Ordering::Equal => {
                    let nv = self.nets.get(net as usize);
                    SelChain::Elem {
                        signed: nv.map(|nv| nv.signed),
                        width: nv.map(|nv| nv.width),
                    }
                }
                std::cmp::Ordering::Greater => SelChain::Bit { net: Some(net) },
            };
        }
        SelChain::Bit { net: Some(net) }
    }

    /// The SELF-DETERMINED width (IEEE §11.6.1, Table 11-21) of a size-cast
    /// operand, answered at the AST — over exactly the leaf kinds
    /// [`Self::ast_ctx_signed`] resolves, and with each leaf's width taken from
    /// the same table the lowering materializes it from (`NetVar.width`,
    /// `const_param_expr_w`'s meta/value rule, a wide const's own width). `None`
    /// wherever a width would be a guess (a select with non-constant bounds, a
    /// multi-dimensional element, a call, a system function, a cast) — the caller
    /// then measures it by lowering, which is exact by construction.
    ///
    /// The number has to be EXACT, not merely safe: the evaluation width decides
    /// what a logical shift brings in and what a division sees, so an
    /// over-estimate is as wrong as an under-estimate (`8'(s8 >> 2)` on `-16` is
    /// `3c` at 8 bits and `fc` at 32).
    fn size_ctx_self_width(&self, e: &ast::Expr) -> Option<u32> {
        use ast::ExprKind as K;
        let rec = |x: &ast::Expr| self.size_ctx_self_width(x);
        match &e.kind {
            K::Paren { inner } => rec(inner),
            K::Binary { op, lhs, rhs } => match op {
                ast::BinOp::Lt
                | ast::BinOp::Le
                | ast::BinOp::Gt
                | ast::BinOp::Ge
                | ast::BinOp::Eq
                | ast::BinOp::Ne
                | ast::BinOp::CaseEq
                | ast::BinOp::CaseNe
                | ast::BinOp::WildEq
                | ast::BinOp::WildNe
                | ast::BinOp::LogAnd
                | ast::BinOp::LogOr => Some(1),
                // a shift / power is as wide as its LEFT operand alone.
                ast::BinOp::Shl
                | ast::BinOp::Shr
                | ast::BinOp::AShl
                | ast::BinOp::AShr
                | ast::BinOp::Pow => rec(lhs),
                _ => Some(rec(lhs)?.max(rec(rhs)?)),
            },
            K::Unary { op, operand } => match op {
                ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot => rec(operand),
                _ => Some(1), // reductions / logical-not
            },
            K::Ternary { then_e, else_e, .. } => Some(rec(then_e)?.max(rec(else_e)?)),
            K::Concat { parts } => {
                let mut sum: u32 = 0;
                for p in parts {
                    sum = sum.checked_add(rec(p)?)?;
                }
                Some(sum)
            }
            K::Replicate { count, value } => {
                let c = u32::try_from(self.const_eval_in_scope(count)?).ok()?;
                let mut sum: u32 = 0;
                for v in value {
                    sum = sum.checked_add(rec(v)?)?;
                }
                c.checked_mul(sum)
            }
            // The same chain rule as the classifier: a BIT is 1 wide (a slice of a
            // multi-packed-dim vector is not claimed), an ELEMENT is its element's
            // width, an element of a dynamic handle and a sub-array are not claimed.
            K::BitSelect { .. } => match self.select_chain(e) {
                SelChain::Bit { net } => net
                    .is_none_or(|n| self.packed_dims.get(&n).is_none_or(|d| d.len() <= 1))
                    .then_some(1),
                SelChain::Elem { width, .. } => width,
                SelChain::Unknown | SelChain::NotAValue => None,
            },
            K::PartSelect { base, msb, lsb, .. } => {
                let K::Ident(p) = &base.kind else {
                    return None;
                };
                let [seg] = p.segments.as_slice() else {
                    return None;
                };
                let net = self.lookup_net_scoped(&seg.name)?;
                if self.net_is_static_array(net)
                    || self.packed_dims.get(&net).is_some_and(|d| d.len() > 1)
                {
                    return None;
                }
                let m = self.const_eval_in_scope(msb)?;
                let l = self.const_eval_in_scope(lsb)?;
                u32::try_from(m.abs_diff(l) + 1).ok()
            }
            K::IndexedPart { width, .. } => u32::try_from(self.const_eval_in_scope(width)?).ok(),
            K::IntLit { kind, raw } => {
                if literal::is_fill_literal(raw, *kind) {
                    // no self width: it takes the context's (§11.6). Zero so it
                    // never widens the maximum.
                    return Some(0);
                }
                literal::parse_int_literal(raw, *kind).map(|c| c.width)
            }
            // A net reads as `Signal{net}` and `ir_bits_of` gives that node the
            // table width (a string's is a dynamic length ⇒ none); a REAL net is
            // 64 there too — the context path refuses it before the width can
            // matter, and answering it here keeps the operand out of the probe,
            // which would lower a nested cast a second time and report its
            // refusal twice. A whole unpacked array has no value here.
            K::Ident(p) if p.segments.len() == 1 => {
                let name = &p.segments[0].name;
                match self.bare_ident_route(name, e.span) {
                    BareIdentRoute::ArrayItem { width, .. } => Some(width),
                    BareIdentRoute::Subst(eid) => (!self.verbatim_actuals.contains(&eid))
                        .then(|| self.ir_bits_of(eid))
                        .flatten(),
                    BareIdentRoute::OutSubst(net) => {
                        if self.net_is_static_array(net) {
                            return None;
                        }
                        self.signal_read_width(net)
                    }
                    BareIdentRoute::Str(_) | BareIdentRoute::Real(_) => None,
                    BareIdentRoute::Wide(cv) => Some(cv.width),
                    BareIdentRoute::Param { guessed: true, .. } => None,
                    BareIdentRoute::Param { v, meta, .. } => Some(Self::param_const_width(v, meta)),
                    BareIdentRoute::Other => {
                        let net = self.lookup_net_scoped(name)?;
                        if self.net_is_static_array(net) {
                            return None;
                        }
                        self.signal_read_width(net)
                    }
                }
            }
            K::PkgScoped { pkg, name } => self.pkg_const_read_width(&pkg.name, &name.name),
            // A nested cast is a self-determined leaf as wide as its target — the
            // classifier does not look through a concat, so `8'({4'(r2*2)} + r)`
            // routes and the inner cast's width has to be answered here, or the
            // probe would lower it a second time and report its refusal twice.
            K::Cast { target, expr } => match target {
                ast::CastTarget::Size(_) | ast::CastTarget::Named(_) => {
                    u32::try_from(self.cast_size_bits(target)?)
                        .ok()
                        .filter(|n| *n >= 1)
                }
                ast::CastTarget::Prim(p) => cast_prim_wsign(*p).map(|(w, _, _)| w),
                ast::CastTarget::Signing { .. } => rec(expr),
            },
            K::SysCall { name, args }
                if matches!(name.name.as_str(), "$signed" | "$unsigned") && args.len() == 1 =>
            {
                rec(&args[0])
            }
            _ => None,
        }
    }

    /// `pkg::NAME` twin of the `Ident` arm of [`Self::size_ctx_self_width`].
    fn pkg_const_read_width(&self, pkg: &str, name: &str) -> Option<u32> {
        if self
            .pkg_real_val
            .get(pkg)
            .is_some_and(|m| m.contains_key(name))
            || self
                .pkg_str_raw
                .get(pkg)
                .is_some_and(|m| m.contains_key(name))
        {
            return None;
        }
        if let Some(cv) = self.pkg_wide_bits.get(pkg).and_then(|m| m.get(name)) {
            return Some(cv.width);
        }
        if let Some(&v) = self.pkg_consts.get(pkg).and_then(|m| m.get(name)) {
            let meta = self
                .pkg_const_meta
                .get(pkg)
                .and_then(|m| m.get(name))
                .copied();
            return Some(Self::param_const_width(v, meta));
        }
        let net = *self.pkg_vars.get(pkg)?.get(name)?;
        if self.net_is_static_array(net) {
            return None;
        }
        self.signal_read_width(net)
    }

    /// The width `ir_bits_of` reports for a `Signal{net}` read — mirrored, not
    /// re-derived: `String` is a dynamic length (none), everything else the
    /// table width floored at 1.
    fn signal_read_width(&self, net: u32) -> Option<u32> {
        let nv = self.nets.get(net as usize)?;
        (nv.kind != ir::NetKind::String).then_some(nv.width.max(1))
    }

    /// The WIDTH a parameter read materializes with — the width half of
    /// [`Self::param_const_signed`], spelled once: `const_param_expr_w` uses the
    /// declared meta width whenever it is ≥ 1, else `const_param_expr` binds the
    /// value at 32 bits when it fits `u32` or `i32`, else at 64.
    pub(crate) fn param_const_width(v: i64, meta: Option<(u32, bool)>) -> u32 {
        match meta {
            Some((w, _)) if w >= 1 => w,
            _ => {
                if u32::try_from(v).is_ok() || i32::try_from(v).is_ok() {
                    32
                } else {
                    64
                }
            }
        }
    }

    /// §4.5.212: lower a size-cast operand in CONTEXT WIDTH `n`, recursing the
    /// context-determined operator structure so each leaf is widened to `n` BEFORE the
    /// operation (so `8'(a*b)` multiplies two 8-bit operands = 45, not a 4-bit 13). The
    /// operand-vs-shift-amount / self-determined split mirrors `lower_expr_ctx`; `ext`
    /// (the whole operand's sign, from `ast_ctx_signed`) governs every leaf's extension.
    /// PRIVATE on purpose: the `size_cast_real_reported` invariant requires every
    /// entry to go through [`Self::lower_size_ctx_entry`], which saves and restores
    /// the flag. A `pub(crate)` caller could set it with nothing to restore it, and
    /// a leaked `true` SUPPRESSES a later cast's diagnostic — loud→silent.
    fn lower_size_ctx(&mut self, e: &ast::Expr, n: u32, ext: bool) -> u32 {
        match &e.kind {
            ast::ExprKind::Paren { inner } => self.lower_size_ctx(inner, n, ext),
            ast::ExprKind::Binary { op, lhs, rhs } => {
                let irop = map_binop(*op);
                match op {
                    ast::BinOp::Add
                    | ast::BinOp::Sub
                    | ast::BinOp::Mul
                    | ast::BinOp::BitAnd
                    | ast::BinOp::BitOr
                    | ast::BinOp::BitXor
                    | ast::BinOp::BitXnor => {
                        // both operands context-determined at n.
                        let l = self.lower_size_ctx(lhs, n, ext);
                        let r = self.lower_size_ctx(rhs, n, ext);
                        self.push_expr(ir::Expr::Binary {
                            op: irop,
                            lhs: l,
                            rhs: r,
                        })
                    }
                    // `/ % >> >>>` — the four whose value depends on bits ABOVE the
                    // ones they produce. IEEE 1800 §11.8.1 evaluates a context-determined
                    // operand at **max(self, n)**: the context can only WIDEN. For the
                    // operators above that distinction is invisible (their low n bits are
                    // decided by the operands' low n bits, so narrowing is the same
                    // computation), which is why §4.5.212 could pass `n` straight down
                    // and only the widening half was ever measured. Here it is visible in
                    // both directions:
                    //
                    //   NARROW  `2'(k%4)` with `integer k` — at n=2 the divisor `4`
                    //           becomes `0` and the answer is `xx`; both oracles say `11`.
                    //   WIDEN   `8'(b>>1)` with `b = -4'sd8` — the context's sign bits are
                    //           part of the result (`01111100`); at self width it is
                    //           `00000100`. `8'(b/c)` with `c = -4'sd1` overflows at 4 bits
                    //           and not at 8. A div-by-zero's `x` must fill all n bits.
                    //
                    // So neither `n` nor the self width is right on its own. `plain`
                    // is lowered as a WIDTH PROBE — `w` is the only thing read out of
                    // it, on both paths — and then the branch builds the lowering the
                    // standard selects. The probe's nodes are dead: nothing references
                    // them, so no value is evaluated twice and no `$random` draw is
                    // duplicated (measured). ⚠️ They are not free either: the probe
                    // still emits DIAGNOSTICS, so an error inside a cast operand is
                    // reported twice, and its dead nodes are serialized into the
                    // artifact (a depth-32 nest grows the `.velab` 5.7×). Recorded in
                    // ROADMAP §2 — the standing fix is an AST self-width pass that
                    // answers `w` without lowering at all.
                    // (Widening PAST max(self, n) is wrong too: a 32-bit context makes
                    // `5'(s4>>u3)` 30 instead of 6.)
                    // `**` belongs here on the standard's terms — a NEGATIVE exponent
                    // asks whether the base is ±1, a question about the WHOLE base, so
                    // §4.5.316's low-bit-closure argument (which needs b ≥ 0) does not
                    // cover it. §4.5.318 tried the move and measured it as a net loss
                    // (138 fixed, 20 turned silently wrong) because the engine read the
                    // exponent's sign off the BASE; with that root fixed the move is a
                    // pure gain and `**` now sits where the standard puts it.
                    ast::BinOp::Div
                    | ast::BinOp::Mod
                    | ast::BinOp::Shr
                    | ast::BinOp::AShr
                    | ast::BinOp::Pow => {
                        let plain = self.lower_ctx_or_plain(e, n);
                        let w = self.ir_bits_of(plain).unwrap_or(n);
                        if n >= w {
                            // context WIDENS (or matches): operands at n, as before.
                            let l = self.lower_size_ctx(lhs, n, ext);
                            let r = if matches!(op, ast::BinOp::Div | ast::BinOp::Mod) {
                                self.lower_size_ctx(rhs, n, ext)
                            } else {
                                let r = self.lower_expr(rhs); // shift amount is self-determined
                                self.refuse_real_size_operand(r)
                            };
                            self.push_expr(ir::Expr::Binary {
                                op: irop,
                                lhs: l,
                                rhs: r,
                            })
                        } else {
                            // context NARROWS: the node keeps its own WIDTH — but
                            // §11.8.1 still coerces its operands to the EXPRESSION's
                            // signedness, and for `>>>` that changes the operation
                            // itself: an unsigned left operand shifts LOGICALLY
                            // (§11.4.10). A self-determined lowering keeps the operand's
                            // own sign, so `4'(u8 + (s8 >>> 9))` kept the arithmetic fill
                            // and printed `0010` where both oracles say `0011`. `/ % >>`
                            // do not show it because the plain path already applies the
                            // context's unsignedness to them; `>>>` was the one left.
                            // The fill context is `w`, NOT `n`. §11.6 sizes an
                            // unsized fill to the EXPRESSION's width, and this branch
                            // runs only when `w > n`, so `n` is always too narrow here:
                            // `2'(k / '1)` built the divisor as 2 ones instead of 32 and
                            // printed 2 where both oracles print 0.
                            // The refusal is placed before `coerce_sign` for reading
                            // order only. It was load-bearing in the first cut —
                            // `$unsigned(real)` hid the real from every later check —
                            // but the `$signed`/`$unsigned` pass-through arm added to
                            // `expr_is_real` in this same slice makes
                            // `refuse(coerce_sign(x))` fire exactly when `refuse(x)`
                            // does, so swapping them is now provably a no-op (a
                            // reviewer's swap mutation passes every gate). Do not
                            // read this order as a constraint.
                            // §4.5.318: RECURSE at `w` instead of lowering the operand
                            // self-determined. `lower_ctx_or_plain` keeps every INNER
                            // leaf's own sign, so a signed leaf under an unsigned
                            // expression sign-extended and `coerce_sign` — which only
                            // stamps the RESULT — could not undo it:
                            // `4'(P + ((i13 | s4) >> 2))` printed 12 where both oracles
                            // print 0. §11.8.1 coerces EVERY operand in the region, and
                            // the recursion is the only thing that reaches inner ones.
                            let l = self.lower_size_ctx(lhs, w, ext);
                            let l = self.refuse_real_size_operand(l);
                            let l = self.coerce_sign(l, ext);
                            let r = if matches!(op, ast::BinOp::Div | ast::BinOp::Mod) {
                                let r = self.lower_size_ctx(rhs, w, ext);
                                let r = self.refuse_real_size_operand(r);
                                self.coerce_sign(r, ext)
                            } else {
                                let r = self.lower_expr(rhs); // shift amount is self-determined
                                self.refuse_real_size_operand(r)
                            };
                            let narrow_op = if matches!(op, ast::BinOp::AShr) && !ext {
                                ir::BinOp::Shr
                            } else {
                                irop
                            };
                            let _ = plain; // the width probe; nothing references it
                            self.push_expr(ir::Expr::Binary {
                                op: narrow_op,
                                lhs: l,
                                rhs: r,
                            })
                        }
                    }
                    ast::BinOp::Shl | ast::BinOp::AShl => {
                        // base context-determined at n; shift amount SELF-determined.
                        let l = self.lower_size_ctx(lhs, n, ext);
                        let r = self.lower_expr(rhs);
                        let r = self.refuse_real_size_operand(r);
                        self.push_expr(ir::Expr::Binary {
                            op: irop,
                            lhs: l,
                            rhs: r,
                        })
                    }
                    // comparison / logical: a 1-bit self-determined result → extend as a leaf.
                    _ => self.lower_size_leaf(e, n, ext),
                }
            }
            ast::ExprKind::Unary { op, operand } => match op {
                ast::UnOp::Plus | ast::UnOp::Minus | ast::UnOp::BitNot => {
                    let o = self.lower_size_ctx(operand, n, ext);
                    self.push_expr(ir::Expr::Unary {
                        op: map_unop(*op),
                        operand: o,
                    })
                }
                _ => self.lower_size_leaf(e, n, ext),
            },
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                // The condition is self-determined AND may legally be real — it is
                // tested for nonzero, never resized. Deliberately NOT funnelled
                // through `refuse_real_size_operand`: `4'(r ? a : b)` matches
                // iverilog today, including the `-0.0` discriminator (both `0011`).
                let c = self.lower_expr(cond);
                let t = self.lower_size_ctx(then_e, n, ext);
                let f = self.lower_size_ctx(else_e, n, ext);
                self.push_expr(ir::Expr::Ternary {
                    cond: c,
                    then_e: t,
                    else_e: f,
                })
            }
            // a self-determined value (leaf / select / concat / call) → resize to n.
            _ => self.lower_size_leaf(e, n, ext),
        }
    }

    /// §4.5.317: refuse a REAL value that has reached a size-cast OPERAND position.
    /// Every operand [`Self::lower_size_ctx`] builds funnels through here. If one is
    /// allowed through, the resize / `Binary` node below reads the f64 as a bit
    /// vector, so `4'(r*2)` with `r = 7.5` printed `0000` (the true 15 is `1111`)
    /// and `8'(u/r)` printed `00110011` — silently, at exit 0.
    ///
    /// It has to be the funnel and not one site: the first cut guarded only
    /// `lower_size_leaf`, and `4'(a << r)` (shift amount, `lower_expr`) plus
    /// `8'(u / r)` (the `Div` NARROW branch, `lower_ctx_or_plain`) stayed silent —
    /// while the very same `a << r` OUTSIDE a cast was already loud. A second
    /// spelling of "operand" is exactly how the gate lost them.
    ///
    /// The cast arms' own `cast_operand_is_real` catches SOME of these already (it
    /// shares `expr_is_real`, so a `Div`/`Pow`-rooted operand and a bare `4'(r)` were
    /// loud before this) — what it cannot see is an operand whose leaves this
    /// lowering has already wrapped in `Select`/`Concat`, or one rooted at `Mod` /
    /// a shift / a bitwise op, none of which are in that predicate's `Binary` arm.
    ///
    /// ⚠️ SCOPE: this covers every operand the size-context lowering BUILDS, not
    /// every real that can reach a cast. When `ast_ctx_signed` cannot resolve a leaf
    /// (a real param, a real literal, a function return, `$realtime`, `$sqrt`) the
    /// cast arm never enters `lower_size_ctx` at all, and if the operator is also
    /// outside `expr_is_real`'s `Binary` arm the cast is still silent —
    /// `4'(RP ^ '0)` measures at exit 0 where iverilog refuses. That class is
    /// PRE-EXISTING and needs the tree-wide AST self-width/domain pass; ROADMAP §2.
    ///
    /// iverilog rejects all of these (`Cast base expression must be a vector type`),
    /// and it rejects a bare `4'(r)` too — so this is oracle parity, not a new
    /// restriction and not a capability gap. (verilator accepts them with a `REALCVT`
    /// warning and computes `4'(r*2)` = `1111`, so PRE was wrong against it too.)
    fn refuse_real_size_operand(&mut self, x: u32) -> u32 {
        if !self.expr_is_real(x) {
            return x;
        }
        if !self.size_cast_real_reported {
            self.size_cast_real_reported = true;
            self.error(MsgCode::ElabUnsupported, REAL_SIZE_CAST_MSG);
        }
        self.placeholder_expr()
    }

    /// May a size cast take the CONTEXT path over `e`, and with what
    /// `(sign, self width)`? BOTH have to be known: routing an operand whose
    /// width the walk cannot measure hands `lower_size_leaf` a node it resizes
    /// from a fabricated 32 — a hierarchical read, a class field, a verbatim
    /// inline actual and a hierarchical CALL NAME are all placeholders here, and
    /// each one produced `x` where the fill-only path was right (round-6 review:
    /// `16'(PS16 * {u.hf(-8'sd16), 1'b0})` printed `xxxx` for both oracles'
    /// `f640`, because the `Concat` sign arm answers without descending while
    /// the width arm declines). "Route only what the walk can measure" is the
    /// one predicate that covers every such leaf, in every position, without a
    /// syntactic guard to keep in step with the AST.
    pub(crate) fn size_ctx_route(&self, e: &ast::Expr) -> Option<(bool, Option<u32>)> {
        // ONE gate for both walks. An operand holding an opaque leaf keeps the
        // pre-slice SIGN (see `ast_ctx_signed`) and the pre-slice EVALUATION
        // WIDTH (`None` ⇒ `n`): the width walk sizes a select from its own
        // `[msb:lsb]` without looking at the base, so `4'(s8 - u.v[0+:4])`
        // measured 8 and evaluated wider than the pre-slice `n` over a
        // placeholder — `x` where PRE and both oracles agree.
        let opaque = self.has_opaque_leaf(e);
        let ext = self.ctx_signed_impl(e, !opaque)?;
        let w = (!opaque).then(|| self.size_ctx_self_width(e)).flatten();
        Some((ext, w))
    }

    /// The PRE-SLICE answer for a `[i]` select — verbatim: a `[i]` whose base is
    /// DIRECTLY a single-segment ident naming a declared unpacked array carries
    /// that net's sign, everything else is §5.4.1-unsigned. Its own function so
    /// the fallback is provably the old decision, not a re-derivation of it.
    fn pre_slice_elem_signed(&self, e: &ast::Expr) -> bool {
        let ast::ExprKind::BitSelect { base, .. } = &e.kind else {
            return false;
        };
        let mut b = base.as_ref();
        while let ast::ExprKind::Paren { inner } = &b.kind {
            b = inner;
        }
        let ast::ExprKind::Ident(p) = &b.kind else {
            return false;
        };
        let [seg] = p.segments.as_slice() else {
            return false;
        };
        self.lookup_net_scoped(&seg.name)
            .filter(|net| self.net_is_static_array(*net))
            .and_then(|net| self.nets.get(net as usize))
            .is_some_and(|nv| nv.signed)
    }

    /// Does `e` read anything this walk cannot give a width to — a hierarchical
    /// or class-member name, a hierarchical CALL (the callee's own name is a
    /// path, and the args walk alone missed it), or a bare name bound to an
    /// inline actual handed over VERBATIM (`verbatim_actuals`: a frame call, a
    /// class field, a hierarchical net — the actual is a placeholder and the
    /// operand carries no hierarchical spelling at all)? The `Concat` and
    /// `Replicate` sign arms answer `Some(false)` WITHOUT descending, so an
    /// opaque leaf wrapped in braces otherwise walks straight past every guard
    /// (round-6 review: `16'(PS16 * {u.hf(-8'sd16), 1'b0})` and a formal bound
    /// to `u.v` both printed `xxxx` where PRE and both oracles say `f640`).
    fn has_opaque_leaf(&self, e: &ast::Expr) -> bool {
        use ast::ExprKind as K;
        let rec = |x: &ast::Expr| self.has_opaque_leaf(x);
        match &e.kind {
            K::Ident(p) => {
                if p.segments.len() > 1 {
                    return true;
                }
                matches!(
                    self.bare_ident_route(&p.segments[0].name, e.span),
                    BareIdentRoute::Subst(eid) if self.verbatim_actuals.contains(&eid)
                )
            }
            K::Call { name, args } => name.segments.len() > 1 || args.iter().any(rec),
            K::Paren { inner } => rec(inner),
            K::Unary { operand, .. } => rec(operand),
            K::Binary { lhs, rhs, .. } => rec(lhs) || rec(rhs),
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => rec(cond) || rec(then_e) || rec(else_e),
            K::BitSelect { base, index } => rec(base) || rec(index),
            K::PartSelect { base, msb, lsb } => rec(base) || rec(msb) || rec(lsb),
            K::IndexedPart { base, offset, .. } => rec(base) || rec(offset),
            K::Concat { parts } => parts.iter().any(rec),
            K::Replicate { count, value } => rec(count) || value.iter().any(rec),
            K::Cast { expr, .. } => rec(expr),
            K::SysCall { args, .. } => args.iter().any(rec),
            // a package constant or variable is resolved here and has a width
            K::PkgScoped { .. } => false,
            K::IntLit { .. } | K::RealLit { .. } | K::StrLit { .. } | K::TimeLit { .. } => false,
            // a method call, an assignment pattern, a `let` use, a streaming
            // operator — anything this walk does not model: opaque, not a guess.
            _ => true,
        }
    }

    /// The two cast arms' entry into [`Self::lower_size_ctx`]. Scopes the
    /// "already reported" flag to ONE cast (iverilog reports the cast, not each
    /// leaf) while leaving a NESTED cast free to report its own. No early return
    /// inside, so the restore cannot be skipped.
    ///
    /// §11.8.1 / §6.24.1: the operand is evaluated as if assigned to an N-bit
    /// variable, and an assignment's context can only WIDEN — the evaluation
    /// width is `max(N, the operand's own self-determined width)`, never N alone.
    /// Recursing at N computed `8'(13 + (s8 >> 2))` at 8 bits (`s8 = -16`:
    /// `f0 >> 2` = `3c`, `+ 13` = `49`) where the 32-bit literal makes both
    /// oracles shift at 32 bits (`3ffffffc + 13` = `…09`, low byte `09`). The
    /// self width comes from [`Self::size_ctx_self_width`]; where that declines
    /// the recursion stays at N (see below). The cast's own `lower_size_cast(_, N)`
    /// then resizes the result to N.
    pub(crate) fn lower_size_ctx_entry(
        &mut self,
        e: &ast::Expr,
        n: u32,
        ext: bool,
        w: Option<u32>,
    ) -> u32 {
        let outer = std::mem::replace(&mut self.size_cast_real_reported, false);
        // Dev self-check (`VITA_SCW_CHECK=<file>`): ALSO lower the operand plain
        // and log whether the walk agrees with the lowering on the EVALUATION
        // width. ⚠️ The probe is a second lowering, so while the hook is on every
        // diagnostic inside a routed cast is reported twice and a `$random` draw
        // is duplicated — run the suite without it for a diagnostics count.
        if let Some(log) = std::env::var_os("VITA_SCW_CHECK") {
            use std::io::Write;
            let plain = self.lower_ctx_or_plain(e, n);
            let pw = (!self.expr_is_real(plain))
                .then(|| self.ir_bits_of(plain))
                .flatten();
            let cwd = std::env::current_dir()
                .map(|d| d.display().to_string())
                .unwrap_or_default();
            let line = match (w, pw) {
                (Some(a), Some(p)) if n.max(p) != n.max(a) => format!(
                    "SCW-MISMATCH ast={a} probe={p} n={n} span={:?} cwd={cwd}\n",
                    e.span
                ),
                (Some(_), _) => "SCW-OK\n".to_string(),
                (None, _) => format!("SCW-NONE probe={pw:?} n={n} span={:?} cwd={cwd}\n", e.span),
            };
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(log)
            {
                let _ = f.write_all(line.as_bytes());
            }
        }
        let m = n.max(w.unwrap_or(0));
        let out = self.lower_size_ctx(e, m, ext);
        self.size_cast_real_reported = outer;
        out
    }

    /// Coerce an already-lowered operand to the enclosing expression's signedness
    /// WITHOUT changing its width — §11.8.1 propagates the sign to every operand even
    /// when the context does not widen, and for `>>>` that decides whether the shift
    /// is arithmetic or logical (§11.4.10). The width-changing twin is `lower_size_leaf`.
    fn coerce_sign(&mut self, x: u32, ext: bool) -> u32 {
        let cur = self.expr_self_signed(x);
        if ext == cur {
            return x;
        }
        let which = if ext {
            ir::SysFuncId::Signed
        } else {
            ir::SysFuncId::Unsigned
        };
        self.push_expr(ir::Expr::SysFunc {
            which,
            args: vec![x],
        })
    }

    /// §4.5.212: resize a self-determined leaf to the cast width `n`, extending with the
    /// operand's overall sign `ext` (NOT the leaf's own sign — §11.8.1 coerces every
    /// leaf to the expression's signedness). Re-stamps `$signed`/`$unsigned` so signed
    /// division / arithmetic-shift / comparison in the enclosing op use the right
    /// semantics (a `Concat`/`Select` extension is otherwise always unsigned).
    /// Coerce an already-lowered operand to the enclosing expression's signedness
    /// WITHOUT changing its width — §11.8.1 propagates the sign to every operand even
    /// when the context does not widen. The width-changing twin is `lower_size_leaf`.
    fn lower_size_leaf(&mut self, e: &ast::Expr, n: u32, ext: bool) -> u32 {
        // §4.5.318: an unsized fill is a leaf whose VALUE depends on the width it is
        // sized to, so a bare `lower_expr` builds it at ONE bit and the resize below
        // then extends that — `2'(P + '1)` added 1 instead of 3 and printed `10`
        // where both oracles print `00`. `lower_ctx_or_plain` is the same call for
        // every non-fill leaf (`expr_contains_fill` is false ⇒ literally
        // `lower_expr`), so this is byte-identical off the fill axis.
        //
        // Sizing to `n` rather than to the whole expression's `max(self, n)` is
        // sound BECAUSE a fill is all-ones/all-zeros: its low `n` bits are the same
        // at every width ≥ n, and §4.5.316 already routes the four operators whose
        // answer depends on bits above `n` through the width probe.
        let x = self.lower_ctx_or_plain(e, n);
        let x = self.refuse_real_size_operand(x);
        let w = self.ir_bits_of(x).unwrap_or(32);
        let resized = match n.cmp(&w) {
            std::cmp::Ordering::Equal => x,
            std::cmp::Ordering::Greater => self.extend_to(x, w, n, ext),
            std::cmp::Ordering::Less => self.select_low(x, n),
        };
        let cur_signed = self.expr_self_signed(resized);
        if ext && !cur_signed {
            self.push_expr(ir::Expr::SysFunc {
                which: ir::SysFuncId::Signed,
                args: vec![resized],
            })
        } else if !ext && cur_signed {
            self.push_expr(ir::Expr::SysFunc {
                which: ir::SysFuncId::Unsigned,
                args: vec![resized],
            })
        } else {
            resized
        }
    }
}
