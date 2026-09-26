//! The inline function lane's formal bind — split out of `inline_fn.rs`
//! (mechanical move).

use super::*;

impl Elaborator<'_> {
    /// The real → integral ASSIGNMENT of `e` into a `w`-bit target of sign
    /// `signed` (IEEE §6.12.2 / §10.7), naming `e` ONCE: `RealToInt` rounds half
    /// away from zero into a 128-bit signed integer, and `resize_inline_assign`
    /// narrows it (`select_low`, one mention) and stamps the target sign. A target
    /// wider than 128 bits is reached through `RealToInt + <w-bit signed 0>`: the
    /// add is context-determined at `w` with both operands signed, so the engine
    /// sign-extends the converted value with a single evaluation, where
    /// `extend_to`'s fill bit would name it a second time. That extension is
    /// exact for |x| < 2^127 only — the node is the LOW 128 bits of the rounded
    /// integer, and a wider formal bound to a larger non-repeatable real keeps
    /// the sign-extended low image (ROADMAP §2).
    pub(crate) fn real_to_int_store(&mut self, e: u32, w: u32, signed: bool) -> u32 {
        let rti = self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::RealToInt,
            args: vec![e],
        });
        let v = if w > 128 {
            let mut zero = make_const_u32(0, w);
            zero.signed = true;
            let cid = self.intern_const(zero);
            let z = self.push_expr(ir::Expr::Const { val: cid });
            self.push_expr(ir::Expr::Binary {
                op: ir::BinOp::Add,
                lhs: rti,
                rhs: z,
            })
        } else {
            rti
        };
        self.resize_inline_assign(v, w, signed)
    }

    /// Bind ONE inline (SSA-fold) actual to its formal.
    ///
    /// The inline path substitutes the actual's ExprId for the formal's NAME, so
    /// nothing about the formal's declared type reaches the body unless it is
    /// applied here. But §13.4.3 makes the formal a VARIABLE OF ITS DECLARED TYPE
    /// and §13.5.3 makes the call an ASSIGNMENT to it, and that assignment carries
    /// three properties at once: the declared WIDTH (§11.6.2 — a wide actual
    /// truncates, a narrow one extends by the ACTUAL's own sign, §11.6.1), the
    /// declared SIGNEDNESS (which the body's arithmetic then reads — it is what
    /// makes `x/3` a signed divide), and 2-state-ness (§6.11.1 — x/z store as 0).
    /// The frame path receives all three for free because its formal IS a net.
    ///
    /// ⚠️ The three are each other's preconditions, and applying a SUBSET is worse
    /// than applying none — §4.5.323 measured that three separate ways (the sign
    /// alone lets the body read un-truncated high bits under the new sign; the
    /// width alone leaves the body reading a truncated value with the actual's
    /// sign). So this is one gate for all three: a TRUSTWORTHY actual width. Every
    /// shape without one keeps the pre-slice tail verbatim rather than guess.
    pub(crate) fn bind_formal_actual(
        &mut self,
        eid: u32,
        ast_actual: Option<&ast::Expr>,
        kind: ast::NetVarKind,
        w: u32,
        formal_signed: bool,
    ) -> u32 {
        // A heap-handle (`string`/class/`event`) or `real` formal is not a bit
        // vector: a bit-resize would corrupt the handle or the IEEE-754 payload, so
        // the kind discriminator must precede any width-based work. A width-0 type
        // has nothing to resize.
        if w == 0 || !ast_kind_is_bit_vector(kind) {
            return eid;
        }
        // ⚠️ And the same question on the ACTUAL side, HERE rather than inside
        // `resize_inline_assign`: step (3) below is OUTSIDE that function, so its
        // real/string guards do not cover the coercion. Leaving it there shredded a
        // `real` actual bound to a 2-state formal — `fint(9.0)` printed 0, and a
        // `longint` formal printed the raw IEEE-754 bits — which is §4.5.323 round
        // 2's exact symptom arriving through a different door.
        //
        // `expr_is_real` alone is not enough: it reads the IR value shape, and a
        // frame `Expr::Call` is opaque to it (the frame's return var is a 64-bit
        // `Reg` net holding the f64 payload, not a `NetKind::Real`). `cast_operand_
        // is_real` is the spelling the sibling bind two hundred lines below already
        // uses for exactly this. ⚠️ Its AST half resolves a BARE single-segment name
        // in `func_table` — so `p::f(0)` and `c.cm()` still reach the coercion while
        // the same callee called bare does not, which is the "recognized by spelling,
        // not by value" shape §4.5.310 named. Widening it touches eight call sites
        // and is its own slice (ROADMAP §2). Converting a real actual to the formal's
        // integer type is a separate gap too — this only refuses to make it a
        // DIFFERENT wrong answer.
        let is_real = match ast_actual {
            Some(a) => self.cast_operand_is_real(a, eid),
            None => self.expr_is_real(eid),
        };
        if is_real {
            // §13.5.3 makes this an ASSIGNMENT to a variable of the formal's
            // declared type, so a real actual ROUNDS and then NARROWS to `w` —
            // the gap the note above named ("Converting a real actual to the
            // formal's integer type is a separate gap too"). It is closed here
            // and NOWHERE ELSE on this path, because the inline path substitutes
            // the actual's ExprId for the formal's NAME: with no formal net, the
            // body would otherwise round at ITS OWN width (`f(300.0)` into an
            // `input byte` gave 300 where both oracles give 44). The IR-0 helper
            // `coerce_real_actual_to_formal` is shared with the frame bind and
            // declines the shapes it may not touch (a formal wider than 64 bits, a
            // non-repeatable actual); a declined REPEATABLE actual keeps the
            // pre-slice answer.
            //
            // An actual that may NOT be repeated — a real-returning call, or any
            // expression containing one (`-rf(0)`, `rf(0) * 2.0`) — takes
            // `real_to_int_store` (`RealToInt`, which names it once, then the
            // formal's width and sign, at any width). Measured: `pb(rf(0))`
            // with `rf` returning 300.7 into a `byte` formal was `012d` in a 16-bit
            // net where both oracles give `002d`. Gated on the VALUE-based
            // `expr_is_real` for the resolver reason `coerce_real_actual_to_formal`
            // documents (an integral callee the AST half calls real must not be
            // round-tripped through f64). The repeatable shapes keep the IR-0
            // conversion below, byte-identical.
            if self.expr_is_real(eid) && !self.expr_is_repeatable(eid) {
                return self.real_to_int_store(eid, w, formal_signed);
            }
            return self.coerce_real_actual_to_formal(eid, w, formal_signed);
        }
        if self.ir_expr_is_string(eid) {
            return eid;
        }
        // ⚠️ A SIGNED result is safe to build and unsafe to CONSUME: it tells every
        // downstream widening resize to sign-FILL, and `extend_to` builds that fill
        // as `Select{Bit, base: e}` — a SECOND mention of the operand (§4.5.320 S1).
        // So an actual that cannot be repeated may not become one.
        // Measured: `function [31:0] sgn(input signed [7:0] x); sgn = x;` called with
        // `$random` drew TWICE (the value came from the second draw and the stream
        // ran one ahead of iverilog's) — the widening happened at the RETURN resize,
        // not at the bind, so gating `extend_to` here would not have caught it.
        let duplicated_downstream = formal_signed || net_kind_is_two_state(kind);
        // ㊁ A TRUSTED width whose actual only fails the repeatability test: the
        // assignment is still applied wherever it can be built from operations that
        // name the actual ONCE — `resize_inline_assign`'s sealed path is exactly
        // that for a narrowing (`select_low`) or equal-width bind, and for a
        // widening of an UNSIGNED actual (a constant zero fill); it then stamps
        // the formal's sign. 2-state-ness is `TwoState`, also one mention.
        // Measured: `ib4(c.l8)` (a `bit [3:0]` formal bound to a 4-state class
        // field) was `..83` and `ibyte(c.lx)` `..83`, both oracles `..03` and
        // `ffffffffffffffffff83`; `bs32($random)` (`bit signed [15:0]`) was
        // `8484d609`, iverilog `ffffd609`. A SIGNED actual WIDENED into the formal
        // needs a sign-fill bit, which is a second mention, so it keeps the
        // pre-slice tail below (a recorded residue).
        if let Some(rw) = self.trusted_self_width(eid) {
            if duplicated_downstream
                && (rw >= w || !self.expr_self_signed(eid))
                && !self.expr_is_repeatable(eid)
            {
                let sized = self.resize_inline_assign(eid, w, formal_signed);
                if net_kind_is_two_state(kind) && self.expr_may_be_unknown(sized) {
                    return self.push_expr(ir::Expr::SysFunc {
                        which: ir::SysFuncId::TwoState,
                        args: vec![sized],
                    });
                }
                return sized;
            }
        }
        if self.trusted_self_width(eid).is_none()
            || (duplicated_downstream && !self.expr_is_repeatable(eid))
        {
            // ㊀ pre-slice tail, verbatim: without a trustworthy actual width — or
            // with a SIGNED actual that may only be named once and must be WIDENED
            // into the formal (㊁ above takes every other non-repeatable shape) —
            // the assignment cannot be applied, so only the NARROW-actual extension
            // survives (its high bits were X before that fix), and by the actual's
            // own sign.
            // `resize_inline_assign` re-derives the same width internally and takes
            // its own un-sealed branch when it is untrustworthy.
            let rw = self.ir_bits_of(eid).unwrap_or(w);
            let out = if rw >= w {
                eid
            } else {
                let actual_signed = self.expr_self_signed(eid);
                self.resize_inline_assign(eid, w, actual_signed)
            };
            self.verbatim_actuals.insert(out);
            // (3) 2-STATE still holds on this tail: `TwoState` names the operand
            // ONCE and keeps its width and sign, so nothing above changes.
            // (`b16(c.lu)` — a `bit [15:0]` formal bound to a 4-state class field
            // holding `8'bx0000111` — read back `00X7` for iverilog's `0007` before
            // either arm existed; it now takes ㊁.)
            if net_kind_is_two_state(kind) && self.expr_may_be_unknown(out) {
                let squashed = self.push_expr(ir::Expr::SysFunc {
                    which: ir::SysFuncId::TwoState,
                    args: vec![out],
                });
                self.verbatim_actuals.insert(squashed);
                return squashed;
            }
            return out;
        }
        // (2.5) A NARROW actual bound to a WIDER 2-state formal: coerce at the
        // actual's width, extend the coerced value, and let `resize_inline_assign`
        // below apply the SEAL at the now-equal width. Both steps name the actual
        // ONCE — `TwoState`, then either a constant zero fill (unsigned actual) or
        // `extend_signed_once` (signed actual). Coercing first and extending after
        // equals extending first and coercing after: the extension bits are a
        // literal 0 or copies of the sign bit, and x/z→0 is a per-bit map.
        //
        // ⚠️ The extension SIGN is taken from `eid`: `expr_self_signed` is the very
        // spelling `resize_inline_assign` uses internally, so the extension
        // direction is unchanged, mirror caveat (ROADMAP §2) included.
        //
        // ⚠️ `trusted_self_width` is `Some` here — the guard immediately above
        // returns when it is not — so `rw` is a DECLARED width and not a fabricated
        // 32.
        let rw = self.trusted_self_width(eid).unwrap_or(w);
        if net_kind_is_two_state(kind) && rw > 0 && w > rw && self.expr_may_be_unknown(eid) {
            let actual_signed = self.expr_self_signed(eid);
            let low = self.two_state_once(eid);
            let ext = if actual_signed {
                self.extend_signed_once(low, w)
            } else {
                let zero = self.const_u32_expr(0, 1);
                self.extend_with_fill(low, zero, w - rw)
            };
            return self.resize_inline_assign(ext, w, formal_signed);
        }
        // (1) WIDTH and (2) SIGN, in ONE primitive. Using a separate primitive per
        // direction is what let §4.5.323 round 2 truncate a `real` actual's f64 bits:
        // `resize_inline_assign` owns the real/string guards, both resize directions
        // and the `$signed`/`$unsigned` tail, and that tail is also the SEAL — a bare
        // `Binary`/`Unary`/`Ternary` actual is context-determined to the engine, so
        // without it the body's own width would propagate back out into the actual.
        let sized = self.resize_inline_assign(eid, w, formal_signed);
        // (3) 2-STATE (§6.11.1). Nothing else on this path drops x/z: the frame
        // path's formal net does it on the store.
        if !net_kind_is_two_state(kind) {
            return sized;
        }
        // `TwoState` names the actual once. It is built only where the actual can
        // CARRY an x or z: the predicate is conservative in the safe direction — an
        // unproven shape is coerced — and it answers false for `TwoState`, so a
        // NESTED coercion is not coerced again.
        if !self.expr_may_be_unknown(eid) {
            return sized;
        }
        let coerced = self.two_state_once(sized);
        // `TwoState` keeps `sized`'s width and sign; a signed formal is re-stamped
        // so the node the body reads is the same seal as before.
        if formal_signed {
            self.push_expr(ir::Expr::SysFunc {
                which: ir::SysFuncId::Signed,
                args: vec![coerced],
            })
        } else {
            coerced
        }
    }
}
