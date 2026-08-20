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
            ast::CastTarget::Size(_) => {
                // `cast_size_bits` owns the size fold (SELF-determined — the size
                // expression has no outer context); a private `const_eval_in_scope`
                // here was a second spelling that widened `(4'd9+4'd8)'(2)` to 17
                // bits while the const domain (post-§2-fix) says 1.
                let n = match self.cast_size_bits(target) {
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

    /// The signedness a cast INHERITS from its operand (§6.24.1) — the
    /// CANONICAL §5.4.1/§5.5 rule, i.e. the same answer the engine's width
    /// table will give the very node being wrapped. `None` = NOT YET KNOWABLE.
    ///
    /// It cannot be `expr_self_signed`: that is elaborate's own hand-written
    /// mirror, and it disagrees with the canonical rule at every leaf whose sign
    /// is a SIDECAR rather than a net flag. It reads UNSIGNED where the rule says
    /// signed at a user function's declared return, a class field, and the whole
    /// int-returning system-function family it simply does not list (`$clog2`,
    /// `$countones`, `$random`, `$fopen`, the file-read family, `$dist_*`, the
    /// string→int methods). The cast then extended by the wrong sign, silently:
    /// `16'(f()+4'sd0)` with `function signed [3:0] f` printed `000d` where both
    /// oracles print `fffd`, `int'(f())` printed `0000000d` for `fffffffd`, and
    /// `4'($clog2(300))` printed `0009` for `fff9`.
    ///
    /// It reads SIGNED where the rule says unsigned at exactly one id, `$stime`
    /// (enumerated over every `SysFuncId`) — see `lower_size_cast` for why the
    /// rule wins there. `$urandom` is NOT in either list: both call it unsigned.
    /// An element-typed `pop` cannot reach a cast (`8'(q.pop_front())` is E3009)
    /// but an array REDUCTION can: `8'(q.sum())` on a `byte signed q[$]` goes
    /// `000000fd` → `fffffffd`.
    fn cast_operand_signed(&mut self, e: u32) -> Option<bool> {
        self.canonical_self_width(e).map(|sw| sw.signed)
    }

    /// The sign to EXTEND a cast operand by, and the sign the cast INHERITS.
    /// `widening` says whether `extend_to` is about to build the sign-fill.
    ///
    /// ⚠️ `extend_to`'s sign fill is `Select{Bit, base: e}` — it names the operand
    /// a SECOND time, and the zero fill does not. So adopting a canonical "signed"
    /// that the mirror called unsigned would, for an IMPURE operand, evaluate it
    /// twice: `16'(fp(0))` called `fp` twice, and `16'(fur(0))` (a `$urandom`
    /// wrapper) assembled its value from the sign bit of one draw and the low bits
    /// of the NEXT, shifting the whole stream. PRE could not reach that because the
    /// mirror called every `Call`-rooted operand unsigned. The repeatability
    /// predicate is the one the index seal already uses for this exact hazard
    /// (its doc even names `byte'($urandom)`), and where it says no we keep the
    /// mirror's answer — the pre-slice behaviour, so nothing moves down a rung.
    /// ⚠️ The fallback must be the MIRROR and not `false`: `16'($signed(f()))` is
    /// signed to the mirror too, and hard-wiring `false` there takes it BELOW PRE
    /// (`fffffffffffffffd` → `000000000000000d`).
    /// An operand the mirror ALREADY called signed was duplicated before this
    /// slice too and this guard deliberately does not change that —
    /// `16'($signed(f()))` calls `f` twice in PRE and in POST. (`16'(sa ** f())`
    /// is NOT such a witness: it routes through `lower_size_ctx_entry` to the
    /// same-width arm and calls `f` once in PRE, POST and iverilog alike.)
    fn cast_extend_signed(&mut self, e: u32, widening: bool) -> bool {
        match self.cast_operand_signed(e) {
            Some(true) if widening && !self.expr_is_repeatable(e) => self.expr_self_signed(e),
            Some(s) => s,
            // NOT YET KNOWABLE (a deferred hierarchical reference is still a
            // placeholder): fall back to the mirror, which is what those
            // operands got before this slice.
            None => self.expr_self_signed(e),
        }
    }

    /// `N'(e)` — width N, INHERITING the operand's signedness (§6.24.1).
    ///
    /// The `$signed`/`$unsigned` tail is not decoration: it is the SEAL. A cast
    /// is a self-determined boundary (§11.8.1) — `8'(a*a)` multiplies two 8-bit
    /// operands no matter how wide the expression around it is — but a bare
    /// `Binary`/`Unary`/`Ternary` node is CONTEXT-determined to the engine, so
    /// an enclosing width propagates straight through it and re-runs the whole
    /// operation wider. The `Equal` arm used to return that node raw and every
    /// consumer wider than N then read a different computation:
    /// `logic [15:0] r; r = 8'(a*a)` with `a = 8'hff` printed `fe01` where
    /// iverilog prints `0001` (measured: 634 of 4032 cells, every one of them
    /// with the destination wider than N — assignment, NBA, continuous assign,
    /// port connection, `case` scrutinee, comparison, wider binary operand and
    /// function argument all leak; a concat part or a `$display` argument does
    /// not, because those positions are self-determined already).
    ///
    /// `$signed`/`$unsigned` is the repo's seal primitive (the index seal in
    /// `packed.rs` uses it for the same reason): the engine evaluates its
    /// operand SELF-determined and only then resizes to the context. The other
    /// two arms were already sealed — `extend_to` builds a `Concat` and
    /// `select_low` a `Select`, both self-determined — so ADDING the tail to them
    /// is a no-op (measured: dropping the added `$unsigned` again moves 0 of
    /// 23,667 cells across the three backends). ⚠️ That is not the same claim as
    /// "the tail does nothing there": dropping it ENTIRELY, `$signed` included,
    /// moves 442 of those cells, because `extend_to`/`select_low` are unsigned and
    /// the `$signed` restamp is what the cast inherits.
    ///
    /// ⚠️ THE SEAL NEEDS THE OPERAND'S OWN WIDTH, AND IT IS NOT ALWAYS KNOWN.
    /// `ir_bits_of` answers `None` for a deferred hierarchical placeholder, a
    /// `string` net, the string-producing system functions, and the element-typed
    /// `pop`/array-reduction family — and the caller then FABRICATES 32. Sealing
    /// on that is a rung DOWN, both directions measured:
    ///   - `32'(u1.s)` with `logic signed [15:0] s` — `w` defaults to 32, so
    ///     only `N == 32` reaches the `Equal` arm, and there the old `return e`
    ///     was LOAD-BEARING: the engine's post-resolve width table sign-extended
    ///     it. Sealing stamped the mirror's `$unsigned` → `0000fffd` for the
    ///     oracles' `fffffffd` (28 cells of a 792-cell hierarchical matrix).
    ///   - `40'(s)` with `string s = "abcd"` — a string net's table width is 0,
    ///     so the `Concat` the widening arm builds has a CANONICAL self-width of
    ///     `N−31`, and `$unsigned` evaluates its operand at exactly that width:
    ///     `"abcd"` became `164` at `N = 40`, banded over `33 ≤ N ≤ 61` (the
    ///     mechanism caps the damage at `N ≤ 62` for any string). The bare
    ///     `Concat` escaped it because `eval_concat` reads the parts' real values.
    ///
    /// So the seal is DECLINED exactly when `ir_bits_of` declines, reproducing
    /// the pre-slice shape. Those operands keep the pre-existing leak (ROADMAP
    /// §2 — the same `ir_bits_of` gap the width probe has). The SIGN being
    /// unknown is NOT a reason to decline: `cast_extend_signed` already falls
    /// back to the mirror there, which is the sign the pre-slice code stamped,
    /// so the seal only materializes a decision that was already being made —
    /// and declining on it as well cost `8'(ua ** u1.k)` (a placeholder EXPONENT
    /// with a known 4-bit base) its fix, `02d9` for iverilog's `00d9`.
    ///
    /// With the width known, this function returns a self-determined N-bit node;
    /// with it fabricated it returns exactly what it returned before the seal.
    pub(crate) fn lower_size_cast(&mut self, e: u32, n: u32) -> u32 {
        let known_w = self.ir_bits_of(e);
        let canon_w = self.canonical_self_width(e).map(|s| s.width);
        let w = known_w.unwrap_or(32);
        // The width is TRUSTWORTHY when `ir_bits_of` answered and the canonical
        // rule does not contradict it. Both halves are load-bearing: `None` is a
        // fabricated 32 (below), and a `Some` can be fabricated too — a class
        // field lowers to `Signal{net: 32-bit HANDLE net}` with its real width in
        // the `class_field_widths` sidecar, so `ir_bits_of` reads the handle's 32
        // and `32'(c.u8 + ua)` landed on the same-width arm and sealed there:
        // `0000` for the oracles' `0100`, and `32'(c.s8 + sp)` even flipped sign.
        // `is_none_or` keeps the case where only the canonical rule declines:
        // `ir_bits_of` answers without recursing into the whole tree wherever
        // IEEE gives the width from one side — `**` and the four shifts (Table
        // 11-21: the LEFT operand) and every comparison / logical op / reduction
        // (a fixed 1) — so a placeholder in the OTHER operand stops the canonical
        // walk while the width stays sound. `8'(ua ** u1.k)` is the measured
        // witness; declining on the sign as well cost that cell its fix once.
        let trusted_w = known_w.is_some() && canon_w.is_none_or(|c| c == w);
        let signed_op = self.cast_extend_signed(e, n > w);
        let resized = match n.cmp(&w) {
            // Same width: no resize node — the seal below is the whole job.
            std::cmp::Ordering::Equal => e,
            // Extend: sign-extend iff the operand is signed (§6.24.1), 4-state-
            // preserving (a bitwise `| 0` would corrupt Z→X).
            std::cmp::Ordering::Greater => self.extend_to(e, w, n, signed_op),
            // Truncate to the low N bits (Select is unsigned).
            std::cmp::Ordering::Less => self.select_low(e, n),
        };
        if !trusted_w {
            // Width fabricated ⇒ no seal. The tail SHAPE is the pre-slice one (the
            // `Equal` arm returned `e` with no stamp at all); the sign INPUT is
            // still the canonical one, which is a fix in its own right here —
            // `8'(q.sum())` on a `byte signed q[$]` goes `000000fd` → `fffffffd`.
            return if signed_op && n != w {
                self.push_expr(ir::Expr::SysFunc {
                    which: ir::SysFuncId::Signed,
                    args: vec![resized],
                })
            } else {
                resized
            };
        }
        let which = if signed_op {
            ir::SysFuncId::Signed
        } else {
            ir::SysFuncId::Unsigned
        };
        self.push_expr(ir::Expr::SysFunc {
            which,
            args: vec![resized],
        })
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
            // The OPERAND's sign comes from the canonical rule for the same
            // reason the size cast's does (`cast_extend_signed`, which also owns
            // the operand-repeat guard) — the mirror called a signed function
            // return unsigned and `int'(f())` then zero-extended −3 to
            // `0000000d`. This arm always widens, hence the literal `true`.
            std::cmp::Ordering::Greater => {
                let signed_op = self.cast_extend_signed(e, true);
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
    /// and the §4.1a format-string check).
    ///
    /// The RULE lives in `sim_ir::realness` because the ENGINE needs the same
    /// answer — §11.8.1 makes the integral-to-real conversion boundary
    /// self-determined, so the engine has to know a binary is REAL before it
    /// evaluates either operand, and a second spelling of "what is real" is how
    /// the two crates drift. This is the elaborate-side driver: it recurses,
    /// because its arena is still growing while it lowers.
    pub(crate) fn expr_is_real(&self, eid: u32) -> bool {
        let cx = ir::realness::RealnessCtx {
            exprs: &self.exprs,
            consts: &self.consts,
            nets: &self.nets,
            real_elem_dyn_nets: &self.real_elem_dyn_nets,
            func_ret_is_real: &|f: u32| {
                self.func_metas
                    .get(f as usize)
                    .and_then(|m| self.nets.get((m.base_net + m.return_slot) as usize))
                    .is_some_and(|n| matches!(n.kind, ir::NetKind::Real))
            },
        };
        ir::realness::expr_is_real_node(&cx, &|id| self.expr_is_real(id), eid)
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
