//! casts — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// `(width, signed, two_state)` of a primitive casting type — the ONE table shared
/// by the lowering ([`Elaborator::lower_prim_cast`]) and the integer const domain
/// (`const_eval_cast`), so a cast can never fold to one type at elaborate time and
/// lower as another. `Real` has no integral shape and yields None.
pub(crate) fn cast_prim_wsign(p: ast::CastPrim) -> Option<(u32, bool, bool)> {
    use ast::CastPrim as P;
    Some(match p {
        P::Int => (32, true, true),
        P::Integer => (32, true, false),
        P::Byte => (8, true, true),
        P::Shortint => (16, true, true),
        P::Longint => (64, true, true),
        P::Bit => (1, false, true),
        P::Logic | P::Reg => (1, false, false),
        P::Time => (64, false, false),
        P::Real => return None,
    })
}

impl Elaborator<'_> {
    // ── SV static cast `casting_type'(expr)` (IEEE 1800 §6.24) ──────────────
    // Lowered entirely to EXISTING IR (IR-0; format_version unchanged). Numeric,
    // size, and signing casts are iverilog-pinned; class/typedef-name casts have
    // no oracle yet → loud-reject (correct-or-loud, never silent-wrong).
    pub(crate) fn lower_cast(&mut self, target: &ast::CastTarget, operand: &ast::Expr) -> u32 {
        match target {
            // signed'(e) / unsigned'(e): PRESERVE width, flip the sign attribute.
            ast::CastTarget::Signing { signed } => {
                let e = self.lower_expr(operand);
                if self.cast_operand_is_real(operand, e) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "signed'/unsigned' cast is not defined on a real operand",
                    );
                    return self.placeholder_expr();
                }
                let which = if *signed {
                    ir::SysFuncId::Signed
                } else {
                    ir::SysFuncId::Unsigned
                };
                self.push_expr(ir::Expr::SysFunc {
                    which,
                    args: vec![e],
                })
            }
            // N'(e): result is N bits; signedness INHERITED from the operand.
            ast::CastTarget::Size(w_expr) => {
                let n = match self.const_eval_in_scope(w_expr) {
                    Some(n) if n >= 1 && (n as u64) <= MAX_NET_WIDTH => n as u32,
                    _ => {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "size cast width must be a positive constant expression",
                        );
                        return self.placeholder_expr();
                    }
                };
                // §11.6: the cast width N is the operand's context, so a fill
                // literal (`'1`/`'x`/`'z`) grows to N bits (a bare `lower_expr` sizes
                // it to 1 bit, then `lower_size_cast` zero-extends it = silent-wrong:
                // `8'('1)` gave `01` instead of `ff`). Byte-identical for a non-fill.
                // §4.5.212: when the operand is a context-determined OPERATION
                // (`8'(a*b)`, `6'(a<<1)`), N is the context for the WHOLE operation, so
                // the arithmetic must run at N bits (`lower_size_ctx`), not at the
                // operands' self-width then resize (which lost the carry). The sign of
                // every leaf follows the operand's overall sign (`ast_ctx_signed`); if
                // that can't be resolved here the fill-only path is kept (no regression).
                let e = match (
                    Self::is_size_ctx_operation(operand),
                    self.ast_ctx_signed(operand),
                ) {
                    (true, Some(ext)) => self.lower_size_ctx_entry(operand, n, ext),
                    _ => self.lower_ctx_or_plain(operand, n),
                };
                if self.cast_operand_is_real(operand, e) {
                    self.error(MsgCode::ElabUnsupported, REAL_SIZE_CAST_MSG);
                    return self.placeholder_expr();
                }
                self.lower_size_cast(e, n)
            }
            ast::CastTarget::Prim(p) => self.lower_prim_cast(*p, operand),
            // `name'(e)`: a bare single identifier that folds to a constant
            // parameter/localparam is a legal SIZE cast `W'(e)` (§6.24 casting_type
            // = constant_primary). Otherwise it is a typedef/class NAME cast, which
            // has no oracle yet → loud. `const_eval_in_scope` folds ONLY genuine
            // constants (a net/typedef/class name yields None), so correct-or-loud
            // is preserved.
            ast::CastTarget::Named(path) => {
                if path.segments.len() == 1 {
                    let id_expr = ast::Expr {
                        kind: ast::ExprKind::Ident(path.clone()),
                        span: path.span,
                    };
                    if let Some(n) = self.const_eval_in_scope(&id_expr) {
                        if n >= 1 && (n as u64) <= MAX_NET_WIDTH {
                            // fill literal grows to the cast width N (see Size arm).
                            // §4.5.212: a context-determined operation runs at N bits.
                            let e = match (
                                Self::is_size_ctx_operation(operand),
                                self.ast_ctx_signed(operand),
                            ) {
                                (true, Some(ext)) => {
                                    self.lower_size_ctx_entry(operand, n as u32, ext)
                                }
                                _ => self.lower_ctx_or_plain(operand, n as u32),
                            };
                            if self.cast_operand_is_real(operand, e) {
                                self.error(MsgCode::ElabUnsupported, REAL_SIZE_CAST_MSG);
                                return self.placeholder_expr();
                            }
                            return self.lower_size_cast(e, n as u32);
                        }
                    }
                }
                // A.5: a single-seg path naming a CLASS is a class cast `Base'(d)`
                // (§6.24.2). v1 supports the UP-cast (and identity): the target is
                // the operand's class or an ANCESTOR. An up-cast is a pure IDENTITY
                // on the handle value — the heap object keeps its concrete class_id,
                // virtual dispatch reads the dynamic type (the static cast type is
                // irrelevant), and the static type for later member access rides on
                // the DESTINATION net's own `net_class`, not on this lowered value.
                // So we just return the operand handle UNCHANGED. A down-cast /
                // unrelated cast / unresolvable operand is loud (correct-or-loud:
                // a cast we cannot validate must not silently pass).
                if path.segments.len() == 1 && self.class_table.contains_key(&path.segments[0].name)
                {
                    let target = path.segments[0].name.clone();
                    match self.operand_static_class(operand) {
                        Some(op_class)
                            if target == op_class || self.class_is_ancestor(&target, &op_class) =>
                        {
                            // Legal up-cast / identity → handle value unchanged.
                            return self.lower_expr(operand);
                        }
                        Some(op_class) if self.class_is_ancestor(&op_class, &target) => {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "a static class DOWN-cast `Derived'(base)` is outside the v1 \
                                 cast scope (only up-casts to a base class are supported)",
                            );
                            return self.placeholder_expr();
                        }
                        Some(_) => {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "an UNRELATED class cast (target is neither the operand's class \
                                 nor a base of it) is illegal (IEEE §6.24.2)",
                            );
                            return self.placeholder_expr();
                        }
                        None => {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "a class cast `Base'(expr)` requires an operand whose static \
                                 class is resolvable (a handle variable, `this`, or a handle \
                                 field) so the cast relationship can be validated",
                            );
                            return self.placeholder_expr();
                        }
                    }
                }
                self.error(
                    MsgCode::ElabUnsupported,
                    "typedef/class cast `name'(expr)` is outside the v1 cast scope \
                     (int'/byte'/…/N'/signed' casts are supported)",
                );
                self.placeholder_expr()
            }
        }
    }

    /// `N'(e)` — width N, INHERITING the operand's signedness (§6.24.1).
    pub(crate) fn lower_size_cast(&mut self, e: u32, n: u32) -> u32 {
        let w = self.ir_bits_of(e).unwrap_or(32);
        let signed_op = self.expr_self_signed(e);
        let resized = match n.cmp(&w) {
            // Same width: `e` already carries its own (inherited) sign — return it.
            std::cmp::Ordering::Equal => return e,
            // Extend: sign-extend iff the operand is signed (§6.24.1), 4-state-
            // preserving (a bitwise `| 0` would corrupt Z→X).
            std::cmp::Ordering::Greater => self.extend_to(e, w, n, signed_op),
            // Truncate to the low N bits (Select is unsigned).
            std::cmp::Ordering::Less => self.select_low(e, n),
        };
        // A size cast INHERITS the operand's signedness; Concat/Select are
        // unsigned, so re-stamp $signed when the operand was signed.
        if signed_op {
            self.push_expr(ir::Expr::SysFunc {
                which: ir::SysFuncId::Signed,
                args: vec![resized],
            })
        } else {
            resized
        }
    }

    /// `keyword'(e)` — a primitive-type cast. Width/sign/state come from the named
    /// type; the EXTEND direction follows the OPERAND's sign (engine-resized),
    /// while the RESULT sign is the target's. 2-state targets coerce X/Z→0.
    pub(crate) fn lower_prim_cast(&mut self, p: ast::CastPrim, operand: &ast::Expr) -> u32 {
        use ast::CastPrim as P;
        // real target: real'(real)=identity; real'(integral)=$itor.
        if matches!(p, P::Real) {
            let e = self.lower_expr(operand);
            if self.cast_operand_is_real(operand, e) {
                return e;
            }
            return self.push_expr(ir::Expr::SysFunc {
                which: ir::SysFuncId::Itor,
                args: vec![e],
            });
        }
        let (tw, tsigned, t2state) = cast_prim_wsign(p)
            .expect("`Real` is handled above; every other prim has a table entry");
        // The target width `tw` is the operand's context, so a fill literal grows
        // to `tw` bits (a bare `lower_expr` sizes it to 1 bit, then the resize below
        // zero-extends it = silent-wrong: `int'('1)` gave `00000001`, not all-ones).
        // Byte-identical for a non-fill operand.
        let e = self.lower_ctx_or_plain(operand, tw);
        // real operand → integral target: round half away from zero, then narrow.
        if self.cast_operand_is_real(operand, e) {
            return self.lower_real_to_int_cast(e, tw, tsigned, t2state);
        }
        // integral operand: resize to the target width (sign-extend per the
        // OPERAND's sign), coerce X/Z for 2-state, then stamp the target sign.
        let w = self.ir_bits_of(e).unwrap_or(32);
        let resized = match tw.cmp(&w) {
            std::cmp::Ordering::Equal => e,
            // Sign-extend iff the operand is signed (§6.24/§11.6.1); 4-state-
            // preserving Concat (a `| 0` would zero-extend a signed operand AND
            // corrupt Z→X — the two extend-path silent-wrongs the hunt found).
            std::cmp::Ordering::Greater => {
                let signed_op = self.expr_self_signed(e);
                self.extend_to(e, w, tw, signed_op)
            }
            std::cmp::Ordering::Less => self.select_low(e, tw),
        };
        let coerced = if t2state {
            self.coerce_two_state(resized, tw)
        } else {
            resized
        };
        let which = if tsigned {
            ir::SysFuncId::Signed
        } else {
            ir::SysFuncId::Unsigned
        };
        self.push_expr(ir::Expr::SysFunc {
            which,
            args: vec![coerced],
        })
    }

    /// real → integral cast: ROUND HALF AWAY FROM ZERO (§6.24.1), NOT `$rtoi`
    /// truncation. `round = $rtoi(e + (e >= 0.0 ? 0.5 : -0.5))`. `$rtoi` yields a
    /// 32-bit int; a 33..=64-bit target (`longint'`/`time'`) splits the rounded
    /// real into hi/lo 32-bit words in the REAL domain and concatenates them
    /// (IR-0, bit-exact for every f64-representable value in range — both vita
    /// and iverilog share the 53-bit f64 mantissa, so the differential stays in
    /// parity beyond 2^53). A >64-bit target cannot arise from a primitive cast.
    pub(crate) fn lower_real_to_int_cast(
        &mut self,
        e: u32,
        tw: u32,
        tsigned: bool,
        _t2state: bool,
    ) -> u32 {
        if tw > 64 {
            self.error(
                MsgCode::ElabUnsupported,
                "real→integer cast wider than 64 bits is outside the cast scope",
            );
            return self.placeholder_expr();
        }
        let zero_r = self.real_const_expr("0.0");
        let half_p = self.real_const_expr("0.5");
        let half_p2 = self.real_const_expr("0.5");
        let half_n = self.push_expr(ir::Expr::Unary {
            op: ir::UnOp::Minus,
            operand: half_p2,
        });
        let ge = self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::Ge,
            lhs: e,
            rhs: zero_r,
        });
        let adj = self.push_expr(ir::Expr::Ternary {
            cond: ge,
            then_e: half_p,
            else_e: half_n,
        });
        let sum = self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::Add,
            lhs: e,
            rhs: adj,
        });
        // 33..=64-bit target: decompose the round-half-away integer of `e` into a
        // high and low 32-bit word in the real domain. We must NOT reuse `sum`
        // (= e±0.5): for an exactly-representable ODD integer `e` with |e| in
        // [2^52, 2^53) (f64 ulp = 1.0), `e+0.5` rounds to even = `e+1`, so a
        // floor/ceil of `sum` is off by one (a CRITICAL silent-wrong the hunt
        // found). Instead compute the integer part `te = trunc-toward-zero(e)`
        // and the fractional part `frac = e - te ∈ (-1,1)`, then round HALF AWAY
        // FROM ZERO with an exact ±1 bump: `ts = te + (frac>=0.5 ? 1 : frac<=-0.5
        // ? -1 : 0)`. For |e| >= 2^52 `e` is already integer-valued so `frac = 0`
        // and `ts = e` exactly. All ops stay on integer-valued reals (< 2^63), so
        // `hi = $rtoi($floor(ts/2^32))`, `lo = $rtoi(ts - hi_real*2^32)` and the
        // `{hi, lo}` join (parts[0]=MSB) reconstruct the 64-bit two's-complement
        // value exactly. iverilog `longint'`/`time'`-identical across small/
        // fractional/negative/>2^31/odd-in-[2^52,2^53)/min/max sweeps.
        if tw > 32 {
            // te = trunc-toward-zero(e) (`ge` = `e >= 0`, computed above).
            let floor_e = self.push_expr(ir::Expr::SysFunc {
                which: ir::SysFuncId::Floor,
                args: vec![e],
            });
            let ceil_e = self.push_expr(ir::Expr::SysFunc {
                which: ir::SysFuncId::Ceil,
                args: vec![e],
            });
            let te = self.push_expr(ir::Expr::Ternary {
                cond: ge,
                then_e: floor_e,
                else_e: ceil_e,
            });
            let frac = self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Sub,
                lhs: e,
                rhs: te,
            }); // (-1, 1)
            let p_half = self.real_const_expr("0.5");
            let n_half = self.real_const_expr("-0.5");
            let one_r = self.real_const_expr("1.0");
            let neg_one_r = self.real_const_expr("-1.0");
            let zero_bump = self.real_const_expr("0.0");
            let ge_half = self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Ge,
                lhs: frac,
                rhs: p_half,
            });
            let le_nhalf = self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Le,
                lhs: frac,
                rhs: n_half,
            });
            // bump = frac<=-0.5 ? -1 : 0   (inner), then frac>=0.5 ? 1 : inner.
            let inner = self.push_expr(ir::Expr::Ternary {
                cond: le_nhalf,
                then_e: neg_one_r,
                else_e: zero_bump,
            });
            let bump = self.push_expr(ir::Expr::Ternary {
                cond: ge_half,
                then_e: one_r,
                else_e: inner,
            });
            let ts = self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Add,
                lhs: te,
                rhs: bump,
            }); // integer-valued, round-half-away(e), EXACT
            let two32 = self.real_const_expr("4294967296.0");
            let two32b = self.real_const_expr("4294967296.0");
            let quot = self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Div,
                lhs: ts,
                rhs: two32,
            });
            let floor_q = self.push_expr(ir::Expr::SysFunc {
                which: ir::SysFuncId::Floor,
                args: vec![quot],
            }); // real, integer-valued
            let hi = self.push_expr(ir::Expr::SysFunc {
                which: ir::SysFuncId::Rtoi,
                args: vec![floor_q],
            }); // 32-bit: high word bit pattern
            let prod = self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Mul,
                lhs: floor_q,
                rhs: two32b,
            }); // real = hi * 2^32
            let lo_real = self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Sub,
                lhs: ts,
                rhs: prod,
            }); // real in [0, 2^32)
            let lo = self.push_expr(ir::Expr::SysFunc {
                which: ir::SysFuncId::Rtoi,
                args: vec![lo_real],
            }); // 32-bit: low word bit pattern
                // {hi, lo}: parts[0] is the MSB half (high 32 bits).
            let combined = self.push_expr(ir::Expr::Concat {
                parts: vec![hi, lo],
            }); // 64-bit
            let sel = if tw < 64 {
                self.select_low(combined, tw)
            } else {
                combined
            };
            let which = if tsigned {
                ir::SysFuncId::Signed
            } else {
                ir::SysFuncId::Unsigned
            };
            return self.push_expr(ir::Expr::SysFunc {
                which,
                args: vec![sel],
            });
        }
        let rounded = self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::Rtoi,
            args: vec![sum],
        }); // 32-bit signed
        if tw == 32 {
            return if tsigned {
                rounded
            } else {
                self.push_expr(ir::Expr::SysFunc {
                    which: ir::SysFuncId::Unsigned,
                    args: vec![rounded],
                })
            };
        }
        // narrow ≤32-bit target, stamping the target sign.
        let sel = self.select_low(rounded, tw);
        let which = if tsigned {
            ir::SysFuncId::Signed
        } else {
            ir::SysFuncId::Unsigned
        };
        self.push_expr(ir::Expr::SysFunc {
            which,
            args: vec![sel],
        })
    }

    /// True when the cast operand is real-valued. Reuses the IR `expr_is_real` for
    /// the common shapes, and additionally recognizes a user real-returning function
    /// call REACHABLE through the real-propagating positions (unary `+/-`, real
    /// arithmetic, ternary, parens) — the IR `Call` node carries no real flag, so
    /// `expr_is_real` alone mis-treats `int'(real_fn())` / `int'(-real_fn())` as
    /// integral (a bit-reinterpret) instead of routing through the real→int round
    /// path. The engine itself evaluates the real arithmetic correctly (the return
    /// net is `NetKind::Real`); only this elaborate routing decision needs the call.
    pub(crate) fn cast_operand_is_real(&self, operand: &ast::Expr, eid: u32) -> bool {
        self.expr_is_real(eid) || self.ast_has_real_call(operand)
    }

    /// Static real-ness of an already-lowered ExprId (for §6.2 illegality gates
    /// and the §4.1a format-string check). Real-typed iff it is a real const, a
    /// real net read, a `+`/`-` of a real, a `+ - * /` with a real operand, a
    /// ternary with a real branch, or a real-producing system function.
    pub(crate) fn expr_is_real(&self, eid: u32) -> bool {
        match self.exprs.get(eid as usize) {
            Some(ir::Expr::Const { val }) => self
                .consts
                .get(*val as usize)
                .is_some_and(|c| matches!(c.repr, ir::ConstRepr::Real)),
            Some(ir::Expr::Signal { net, .. }) => {
                self.nets
                    .get(*net as usize)
                    .is_some_and(|n| matches!(n.kind, ir::NetKind::Real))
                    // r19/S1: a `real d[]` ELEMENT. The net is `NetKind::DynArray`,
                    // so the kind test above cannot see it. `real_elem_dyn_nets` is
                    // the set that makes the engine store those elements as f64 in
                    // the first place, so it cannot claim a net the engine does not
                    // hold as real — the same discriminator the string twin uses.
                    || self.real_elem_dyn_nets.contains(net)
            }
            Some(ir::Expr::Unary { op, operand }) => {
                matches!(op, ir::UnOp::Plus | ir::UnOp::Minus) && self.expr_is_real(*operand)
            }
            Some(ir::Expr::Binary { op, lhs, rhs }) => {
                // `**` is real-propagating (§11.4.4); the AST-side twin
                // `ast_has_real_call` carries the identical list — keep them in step.
                matches!(
                    op,
                    ir::BinOp::Add
                        | ir::BinOp::Sub
                        | ir::BinOp::Mul
                        | ir::BinOp::Div
                        | ir::BinOp::Pow
                ) && (self.expr_is_real(*lhs) || self.expr_is_real(*rhs))
            }
            Some(ir::Expr::Ternary { then_e, else_e, .. }) => {
                self.expr_is_real(*then_e) || self.expr_is_real(*else_e)
            }
            // §4.5.317: `$signed`/`$unsigned` are TRANSPARENT to the value's domain.
            // Without this arm `4'($signed(r)*2)` printed `0000` while the very same
            // `$signed(r)*2` with no cast printed the real-domain 15 — two answers to
            // one expression inside one design. It has to live HERE and not in a
            // cast-local wrapper: the wrappers appear UNDER the arithmetic
            // (`Binary{Mul, $signed(r), 2}`), so only the recursion can see them.
            // ANY real argument counts, not just the first: vita currently accepts a
            // two-argument `$signed(r, a)` (pre-existing, ROADMAP §2 — iverilog says
            // "takes exactly one(1) argument"), and keying on `args[0]` made the
            // refusal depend on which slot the real landed in.
            Some(ir::Expr::SysFunc {
                which: ir::SysFuncId::Signed | ir::SysFuncId::Unsigned,
                args,
            }) => args.iter().any(|a| self.expr_is_real(*a)),
            Some(ir::Expr::SysFunc { which, args }) => {
                matches!(
                    which,
                    ir::SysFuncId::Realtime
                        | ir::SysFuncId::Itor
                        | ir::SysFuncId::BitsToReal
                        // r19/S1: `.atoreal()` returns `Value::from_f64` and was
                        // absent here, so `v[s.atoreal()]` read the f64 BIT PATTERN
                        // as an index — reachable with no `real` keyword in sight.
                        | ir::SysFuncId::StrAtoreal
                ) || real_math_arity(*which).is_some()
                    // `.sum()`/`.product()` fold with `arith`, which stays in the
                    // real domain when the elements are real.
                    || (matches!(which, ir::SysFuncId::ArrSum | ir::SysFuncId::ArrProduct)
                        && args.first().is_some_and(|a| self.expr_is_real(*a)))
            }
            // r19/S1: a real-returning function. Every consumer of this predicate is
            // a "must be integral" gate, so a missing arm here is a silent-wrong at
            // ~40 call sites at once — fixing the sites without fixing the predicate
            // left all of them open. Conservative: a call whose callee is known to
            // return a real is real; an unknown callee is not claimed.
            Some(ir::Expr::Call { func, .. }) => self
                .func_metas
                .get(*func as usize)
                .and_then(|m| self.nets.get((m.base_net + m.return_slot) as usize))
                .is_some_and(|n| matches!(n.kind, ir::NetKind::Real)),
            _ => false,
        }
    }

    /// §4.1a STATIC gate: walk the literal format string, pair each conversion
    /// specifier with its positional value-arg, and reject a `%b/%h/%o/%x` (radix)
    /// specifier on a real-typed argument. `%f/%g/%e/%d` on a real are legal.
    pub(crate) fn check_format_real_radix(&mut self, fmt: &str, arg_ids: &[u32]) {
        let mut chars = fmt.chars().peekable();
        let mut argi = 0usize;
        while let Some(c) = chars.next() {
            if c != '%' {
                continue;
            }
            // skip width/precision modifiers (digits and a single '.').
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() || d == '.' {
                    chars.next();
                } else {
                    break;
                }
            }
            let spec = match chars.next() {
                Some(s) => s,
                None => break,
            };
            match spec {
                '%' | 'm' => {} // literal '%' / scope name — consume no arg
                'b' | 'B' | 'h' | 'H' | 'x' | 'X' | 'o' | 'O' => {
                    if arg_ids
                        .get(argi)
                        .copied()
                        .is_some_and(|e| self.expr_is_real(e))
                    {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "binary/hex/octal format not defined on a real argument",
                        );
                    }
                    argi += 1;
                }
                // every other conversion consumes one positional argument.
                _ => {
                    argi += 1;
                }
            }
        }
    }

    /// A real (f64) literal Const expr — e.g. `"100.0"` (N5 coverage real %).
    pub(crate) fn real_const_expr(&mut self, raw: &str) -> u32 {
        let cid = self.intern_const(parse_real_literal(raw));
        self.push_expr(ir::Expr::Const { val: cid })
    }

    /// v9 rank 6: `ok = $cast(dst, src)` (function form) — the `$value$plusargs`
    /// family (the engine writes the `dst` ref arg in the WRITE phase, returns 1).
    /// `dst` must lower to a plain whole-net Signal; `src` is any expression.
    /// iverilog 13.0 does not support `$cast` (no oracle): hand-IEEE §6.24.2,
    /// integral assignment always succeeds. (The task form is in `map_systask`.)
    pub(crate) fn cast_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
    ) -> bool {
        let ast::ExprKind::SysCall { name, args } = &rhs.kind else {
            return false;
        };
        if name.name != "$cast" {
            return false;
        }
        if args.len() != 2 {
            self.error(MsgCode::ElabUnsupported, "$cast takes (dest, source)");
            return true;
        }
        let dst_id = self.lower_expr(&args[0]);
        if !matches!(
            self.exprs.get(dst_id as usize),
            Some(ir::Expr::Signal { word: None, .. })
        ) {
            self.error(
                MsgCode::ElabUnsupported,
                "$cast destination must be a plain integral variable (v9 subset)",
            );
            return true;
        }
        let src_id = self.lower_expr(&args[1]);
        let rhs_id = self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::Cast,
            args: vec![dst_id, src_id],
        });
        self.emit_blocking_intercept(b, lhs, delay, rhs_id);
        true
    }
}
