//! constant-function interpreter — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

pub(crate) fn const_eval_i64_lit(e: &ast::Expr) -> Option<i64> {
    let ast::ExprKind::IntLit { kind, raw } = &e.kind else {
        return None;
    };
    let cv = parse_int_literal(raw, *kind)?;
    if cv.bits.unk.iter().any(|&w| w != 0) {
        return None;
    }
    if cv.bits.val.iter().skip(1).any(|&w| w != 0) {
        return None; // >64-bit literal value — outside the i64 const domain
    }
    let v = cv.bits.val.first().copied().unwrap_or(0);
    let explicit_signed = cv.signed && !matches!(kind, ast::IntLitKind::Decimal);
    if explicit_signed && cv.width >= 1 && cv.width < 64 && (v >> (cv.width - 1)) & 1 == 1 {
        return Some((v | (!0u64 << cv.width)) as i64);
    }
    if cv.width == 64 {
        // A full 64-bit literal (signed OR unsigned) is a 64-bit bit container;
        // reinterpret its bit pattern into the i64 const domain. Without this an
        // unsigned `64'h8000_0000_0000_0000` exceeds `i64::MAX`, so `try_from`
        // below would spuriously reject it as "not foldable" (round-14 V9).
        // Bit-preserving: downstream 64-bit param/compare/mask use is bit-exact,
        // and a magnitude misuse (huge index/bound) surfaces as a negative value
        // that later range checks still reject loudly — so this never converts a
        // loud reject into a silent-wrong. (The narrow-signed branch above still
        // uses `explicit_signed`; only the unsigned MSB-set path is newly folded.)
        return Some(v as i64);
    }
    i64::try_from(v).ok()
}

/// `**` in the i64 const domain. Negative exponents follow the IEEE integer
/// table (1**n=1, (-1)**n=±1, 0**neg undefined → None, else 0); overflow → None.
pub(crate) fn const_pow(a: i64, b: i64, exp_signed: bool) -> Option<i64> {
    // The IEEE negative-exponent table applies only when the exponent REALLY is
    // negative. `64'd0 - 64'd8` is an UNSIGNED subtraction — 18446744073709551608,
    // not −8 — and reading the i64 container's sign instead of the expression's
    // made `3 ** (64'd0 - 64'd8)` answer 0 where iverilog, verilator AND vita's own
    // runtime all answer 926288481.
    let neg = exp_signed && b < 0;
    // A base of 0 or ±1 does not depend on the exponent's MAGNITUDE — only on its
    // parity, which is the low bit of the pattern either way. So these answer for
    // ANY exponent, including the huge unsigned ones the i64 domain cannot carry;
    // gating them behind the domain check below turned `1 ** (64'd0 - 64'd8)` and
    // `(-1) ** (64'd0 - 64'd8)` from both oracles' 1 into a decline.
    match a {
        1 => return Some(1),
        -1 => return Some(if b % 2 == 0 { 1 } else { -1 }),
        // `0 ** 0` is 1; `0 ** positive` is 0; `0 ** negative` is undefined.
        0 => {
            return if b == 0 {
                Some(1)
            } else if neg {
                None
            } else {
                Some(0)
            }
        }
        _ => {}
    }
    if neg {
        return Some(0); // |base| ≥ 2 with a genuinely negative exponent
    }
    // ⚠️⚠️ An "unsigned negative" exponent is a HUGE positive one, so the exact
    // result does not fit the i64 domain — DECLINE, the same discipline `+` and
    // `*` keep here (`3037000500 * 3037000500` is loud too, where both oracles
    // print 145474192).
    //
    // Folding it MODULARLY was tried and reverted. Square-and-multiply mod 2^64
    // gives the right answer at every context of 64 bits or fewer — and both
    // oracles confirmed six such cells — but the module-scope fold has no context
    // width, and a `localparam [127:0] P = 3 ** 41` then zero-extends an
    // ALREADY-TRUNCATED 64-bit value: `resize_bits` cannot restore what the
    // wrapping discarded, so a loud reject became a silent wrong (and at 96 bits,
    // one silent wrong became a different one). Both adversarial lenses landed on
    // it independently. vita's own ENGINE is right there because it works mod
    // 2^128 and switches to exact multi-word kernels past its wide cap.
    //
    // ⇒ the wrapping answer needs a KNOWN context width, which is the width-aware
    // module-scope fold this domain does not have yet — one class with the
    // `+`/`*` overflow, tracked in ROADMAP §2.
    // `b as u64` reads the exponent as the BIT PATTERN it is. (Measured equivalent
    // to `try_from(b)`: anything that fits `u32` is non-negative as an i64 too, so
    // the two decline on exactly the same inputs. Written this way because the
    // pattern reading is the rule, and the next change to this line should start
    // from the rule, not from a coincidence.)
    a.checked_pow(u32::try_from(b as u64).ok()?)
}

/// `<<`/`<<<` in the i64 const domain: value-preserving or None. A shift that
/// loses bits (or lands in the sign bit) would be a silently wrong param value
/// — the round-trip check rejects it loudly. `0 << anything` stays 0.
pub(crate) fn const_shl_i64(a: i64, b: i64) -> Option<i64> {
    if a == 0 {
        return Some(0);
    }
    if !(0..64).contains(&b) {
        return None; // every bit of a non-zero value shifted out / negative amount
    }
    let r = a.checked_shl(b as u32)?;
    if (r >> b) == a {
        Some(r)
    } else {
        None
    }
}

/// Fold one binary operator over two i64 const operands (SIGNED i64 semantics; a
/// folded const carries no x/z, so `===`/`!==`/`==?`/`!=?` collapse to `==`/`!=`).
/// `checked_*` overflow / div-or-mod by zero / a logical `>>` of a negative value
/// (width-dependent) → None → LOUD at the caller. Shared by `const_eval_in_scope`
/// (the plain const domain) and `eval_const_env` (the constant-function interpreter)
/// so both fold identically.
pub(crate) fn const_binop(op: ast::BinOp, a: i64, b: i64) -> Option<i64> {
    match op {
        ast::BinOp::Add => a.checked_add(b),
        ast::BinOp::Sub => a.checked_sub(b),
        ast::BinOp::Mul => a.checked_mul(b),
        ast::BinOp::Div if b != 0 => a.checked_div(b),
        ast::BinOp::Mod if b != 0 => a.checked_rem(b),
        ast::BinOp::Lt => Some((a < b) as i64),
        ast::BinOp::Le => Some((a <= b) as i64),
        ast::BinOp::Gt => Some((a > b) as i64),
        ast::BinOp::Ge => Some((a >= b) as i64),
        ast::BinOp::Eq | ast::BinOp::CaseEq | ast::BinOp::WildEq => Some((a == b) as i64),
        ast::BinOp::Ne | ast::BinOp::CaseNe | ast::BinOp::WildNe => Some((a != b) as i64),
        ast::BinOp::BitAnd => Some(a & b),
        ast::BinOp::BitOr => Some(a | b),
        ast::BinOp::BitXor => Some(a ^ b),
        ast::BinOp::BitXnor => Some(!(a ^ b)),
        ast::BinOp::LogAnd => Some(((a != 0) && (b != 0)) as i64),
        ast::BinOp::LogOr => Some(((a != 0) || (b != 0)) as i64),
        // `**` needs the EXPONENT's signedness, which this signature cannot carry —
        // every reachable site folds it through `const_pow` directly. Declining is
        // fail-closed: a site added later goes loud rather than silently reading an
        // unsigned exponent as a negative one.
        // (A mutant that restores the old signed-reading path here SURVIVES the
        // whole suite — which is the proof that the three sites are exhaustive:
        // nothing reaches this arm. The decline stays as the fail-closed guard for
        // a fourth site added later.)
        ast::BinOp::Pow => {
            debug_assert!(false, "Pow must fold through const_pow");
            None
        }
        // `<<`/`<<<`: value-preserving or None (a shifted-out/overflowing value would
        // be silently wrong). `1<<32` folds wide (4294967296), matching iverilog.
        ast::BinOp::Shl | ast::BinOp::AShl => const_shl_i64(a, b),
        // `>>` (logical): well-defined here only for a ≥ 0 (a negative value's logical
        // shift depends on the operand WIDTH, which this domain does not model).
        ast::BinOp::Shr if a >= 0 => {
            if !(0..64).contains(&b) {
                Some(0)
            } else {
                Some(((a as u64) >> b) as i64)
            }
        }
        // `>>>` (arithmetic): sign-extending; an over-width / negative amount saturates.
        ast::BinOp::AShr => {
            if !(0..64).contains(&b) {
                Some(if a < 0 { -1 } else { 0 })
            } else {
                Some(a >> b)
            }
        }
        // Div/Mod by zero, negative-operand `>>` → non-constant.
        _ => None,
    }
}

/// §4.5.186 constant-function interpreter control flow: fell off the end (`Normal`)
/// or hit a `return [expr]` (`Return`). A break/continue is not modeled (its stmt
/// falls to the interpreter's loud `_` arm), so only these two are needed.
pub(crate) enum ConstFlow {
    Normal,
    Return(Option<i64>),
}

/// Tiny const-evaluator (v1: literals + paren + unary +/-). Evaluate a constant
/// integer expression to `u32`. Anything else (Ident/param, arithmetic) → None
/// (caller substitutes a default + may diagnose). SLOT: param-dependent ranges
/// get a `&params` table here when parameter elaboration lands.
pub(crate) fn const_eval_u32(e: &ast::Expr) -> Option<u32> {
    match &e.kind {
        ast::ExprKind::IntLit { kind, raw } => {
            let cv = parse_int_literal(raw, *kind)?;
            // Reject x/z: a literal with any unknown bit (e.g. `4'dx`) is not a
            // valid constant index/bound/delay — return None so the caller
            // applies its default rather than silently treating x/z as 0.
            // (LOWERING verdict NIT.)
            if cv.bits.unk.iter().any(|&w| w != 0) {
                return None;
            }
            // ⚠️⚠️ FAIL-CLOSED above `u32::MAX`. This used to be `val[0] as u32` — the
            // low 32 bits of the value plane, taken SILENTLY — and every consumer of
            // this helper is a COUNT or an INDEX, so the discarded bits were the ones
            // that decide the answer. Measured at HEAD: `64'hDEAD_BEEF_1234_5678 >>
            // 64'h1_0000_0000` folded to the operand UNSHIFTED (both oracles: 0),
            // `A[2**64]` folded to `A[0]` and answered 1 where the bit is out of range,
            // and `$bits({(2**64+2){8'hA5}})` built a TWO-element replication (16 bits).
            // All three at `errors=0 warnings=0`.
            //
            // ⚠️ The boundary is 2**32, not the 2**64 an external report found by
            // writing 128-bit literals: `>> 64'h1_0000_0000` is a 64-bit literal the
            // i64 domain carries fine, and it was wrong for the same reason.
            //
            // Declining is the fail-closed answer and it is what verilator does
            // (*"Value too wide for 32-bits expected in this context"* for the
            // replication count, an error for the bit-select). A shift wants the
            // CORRECT answer rather than a loud one, so the wide domain asks
            // `fold_shift_count` instead — see `const_wide.rs`.
            if cv.bits.val.iter().skip(1).any(|&w| w != 0) {
                return None;
            }
            u32::try_from(cv.bits.val.first().copied().unwrap_or(0)).ok()
        }
        ast::ExprKind::Paren { inner } => const_eval_u32(inner),
        ast::ExprKind::Unary { op, operand } => {
            let v = const_eval_u32(operand)?;
            match op {
                ast::UnOp::Plus => Some(v),
                ast::UnOp::Minus => Some(v.wrapping_neg()),
                _ => None,
            }
        }
        _ => None,
    }
}

impl Elaborator<'_> {
    pub(crate) fn const_eval_in_scope(&self, e: &ast::Expr) -> Option<i64> {
        match &e.kind {
            ast::ExprKind::IntLit { .. } => const_eval_i64_lit(e),
            // §11.4.12 concatenation / §11.4.12.1 replication of constants:
            // `{4'b0001, 32'b0}`. Each operand is SELF-DETERMINED, so the result is
            // the operands' bits laid end to end and its value depends on every
            // operand's WIDTH — which is why this cannot be the plain i64 fold. It is
            // `const_placement_env` with an EMPTY environment: module scope has no
            // local bindings, so that helper's resolver falls straight through to
            // `lookup_scoped` + `param_meta`.
            //
            // Sharing the helper is what stops the two scopes from answering
            // differently — a constant-function body folded `{2{4'hA}}` and this did
            // not — and it lifts the literal-only limit the hand-rolled loop had: a
            // PARAM operand and a replication both fold now. Those were honest-loud,
            // so this is loud → supported; every shape the old loop admitted takes the
            // same bit placement through the shared folder.
            //
            // Found by elaborating PicoRV32, whose trace-mask localparams are written
            // `localparam [35:0] TRACE_BRANCH = {4'b 0001, 32'b 0};`.
            ast::ExprKind::Concat { .. } | ast::ExprKind::Replicate { .. } => {
                self.const_placement_env(e, &BTreeMap::new(), &ConstWidths::new())
            }
            // G11: a time literal folds to the CURRENT module's time unit. Final ticks =
            // value × 10^(unit_exp − global_prec_exp); the delay path × cur_time_mult, so
            // the folded value is that / cur_time_mult (module units). `None` (loud at the
            // caller) for sub-precision (finer than the design precision / module unit),
            // a negative/real/non-constant value, or overflow.
            ast::ExprKind::TimeLit { num, unit_exp } => {
                let val = self.const_eval_in_scope(num)?;
                if val < 0 {
                    return None;
                }
                let e = *unit_exp as i32 - self.global_prec_exp as i32;
                if e < 0 {
                    return None;
                }
                let ticks = 10i128.checked_pow(e as u32)?.checked_mul(val as i128)?;
                let mult = self.cur_time_mult as i128;
                if mult == 0 || ticks % mult != 0 {
                    return None;
                }
                i64::try_from(ticks / mult).ok()
            }
            ast::ExprKind::Paren { inner } => self.const_eval_in_scope(inner),
            // `!e` yields ONE bit from a SELF-DETERMINED operand (Table 11-21) —
            // `!(4'd15 + 4'd1)` must see the 4-bit 0, not the unlimited 16. The three
            // context-determined unaries keep this walk.
            ast::ExprKind::Unary {
                op: ast::UnOp::LogNot,
                operand,
            } => Some((self.const_int_selfdet(operand)? == 0) as i64),
            ast::ExprKind::Unary { op, operand } => {
                let v = self.const_eval_in_scope(operand)?;
                match op {
                    ast::UnOp::Plus => Some(v),
                    ast::UnOp::Minus => v.checked_neg(),
                    ast::UnOp::BitNot => Some(!v),
                    _ => None,
                }
            }
            // param/genvar reference: single-segment name bound in this scope OR
            // an ENCLOSING one. Walking outward lets a genvar bound at the
            // generate-for's scope (`top.i`) resolve inside the loop body's
            // nested prefix (`top.g[0]`), matching Verilog generate scoping.
            ast::ExprKind::Ident(path) if path.segments.len() == 1 => {
                self.lookup_scoped(&path.segments[0].name)
            }
            // GAP-G: a constant-context element read of a const array parameter
            // (`ROT[i]` — e.g. a generate-scope `localparam R = ROT[g]`). The
            // array is resolved by `const_array_vals_of_base` (module-local,
            // generate-scope, or a package array named `p::ROT` / bare `ROT` via
            // `import p::*`); the index folds; `get` bounds-checks. A non-array
            // base, an out-of-range or negative index, or an array shape not
            // captured (descending / non-zero base / multi-dim / non-foldable
            // element) folds None → loud at the binding site.
            ast::ExprKind::BitSelect { base, index } => {
                let idx = self.const_eval_in_scope(index)?;
                if idx < 0 {
                    return self.const_param_select(e);
                }
                self.const_array_vals_of_base(base)
                    .and_then(|vals| vals.get(idx as usize).copied())
                    // Not a const ARRAY — try the param BIT-select. The array
                    // lookup is first so every shape GAP-G already folded keeps
                    // its exact answer (it declines for a scalar param anyway:
                    // `lookup_scoped` finding the name makes it return None).
                    .or_else(|| self.const_param_select(e))
                    .or_else(|| self.selfdet_bits_i64(e))
            }
            // A constant PART / INDEXED-PART select of a parameter (`W[7:0]`,
            // `W[7 -: 4]`). Without these arms the whole fold declined, and every
            // consumer of a constant bound read `None` — a packed range collapsed
            // to width 1 at exit 0 while both oracles sized it from the selected
            // byte. See `const_select.rs` for the two decline rules.
            ast::ExprKind::PartSelect { .. } | ast::ExprKind::IndexedPart { .. } => {
                // …and, when that declines, the WIDE bit domain. A select of a >64-bit
                // parameter has no answer in `params` at all (the value lives in
                // `wide_param_bits`), which is how `M_ISSUE[n*32 +: 32]` — one port's
                // slice of a per-port vector — stayed loud in a width bound.
                self.const_param_select(e)
                    .or_else(|| self.selfdet_bits_i64(e))
            }
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                // §11.4.11: the CONDITION is self-determined — it takes no width from
                // the arms or from the surrounding context. The real-domain twin
                // (`const_eval_real_in_scope`) already folded it that way; this one
                // did not, so one source line had two answers.
                let c = self.const_int_selfdet(cond)?;
                if c != 0 {
                    self.const_eval_in_scope(then_e)
                } else {
                    self.const_eval_in_scope(else_e)
                }
            }
            ast::ExprKind::SysCall { name, args } if name.name == "$clog2" && args.len() == 1 => {
                // Self-determined, treated-as-unsigned argument — the unlimited
                // fold here answered 4 for `$clog2(4'd15 + 4'd1)` while the
                // constant-function interpreter answered 0 for the same text.
                self.const_clog2_selfdet(&args[0], &BTreeMap::new(), &ConstWidths::new(), 0)
            }
            // §20.10 `$rtoi` TRUNCATES toward zero where a cast rounds, so it gets its
            // own spelling rather than sharing the cast's: `$rtoi(2.9)` is 2 and
            // `int'(2.9)` is 3 (both oracles). A wholly integral argument is already
            // its own truncation and keeps the integer walk.
            ast::ExprKind::SysCall { name, args } if name.name == "$rtoi" && args.len() == 1 => {
                self.const_rtoi_via_real(&args[0])
            }
            // A built-in string method with an integral result (`S.len()`), over a
            // constant string. See `const_string_method`.
            ast::ExprKind::MethodCall { .. } => self.const_string_method(e),
            // v7 P2-D: `pkg::sym` in const contexts.
            ast::ExprKind::PkgScoped { pkg, name } => self
                .pkg_consts
                .get(&pkg.name)
                .and_then(|c| c.get(&name.name))
                .copied(),
            // v7 `$bits` in const contexts (localparam init, range specs): the
            // view subset only — no lowering happens in this domain. A shape
            // it can't see folds None → LOUD at the binding site.
            ast::ExprKind::SysCall { name, args } if name.name == "$bits" && args.len() == 1 => {
                // …and, when the argument is not a view, its SELF-DETERMINED width —
                // a literal, a concatenation, a replication. Without that fallback
                // `$bits(8'h00)` folded to None here, and a packed DECLARATION BOUND
                // built on it silently became ONE BIT (`wire [$bits(8'h00)-1:0] c;`
                // declared a 1-bit net that truncated `8'hA5` to `1`, at exit 0, in
                // every backend). The runtime spelling of the same call already
                // answered 8, so one source line had two answers.
                // …and LAST the wide bit domain, the only one that can size an
                // operand built out of a >64-bit parameter: `$bits(A)` and
                // `$bits(A[63:0])` were E3009 for a 128-bit `A` while vita's own
                // RUNTIME `$bits` of the same text answered 128 and 64. Last in the
                // chain, so nothing that folds today changes route.
                self.bits_of_view(&args[0], true)
                    .or_else(|| self.bits_of_selfdet(&args[0]))
                    .or_else(|| self.wide_selfdet_width(&args[0]))
                    .map(|n| n as i64)
            }
            // A static cast in a constant context (`int'(7)`, `8'(P+1)`). Without
            // this arm `int'(7)` was NOT a foldable constant, so every bound/count
            // site fell back to a weaker folder and degraded SILENTLY (a part-select
            // width collapsed to 1, a replication count to 0).
            ast::ExprKind::Cast { target, expr } => self.const_eval_cast(target, expr),
            ast::ExprKind::Binary { op, lhs, rhs } => {
                // §11.6.1 Table 11-21: a comparison / equality / logical operator
                // delivers ONE bit and sizes its operands against EACH OTHER, so the
                // whole node is a SELF-DETERMINED position — the surrounding context
                // gives it nothing, and this width-unlimited walk is simply the wrong
                // evaluator. It answered `(4'd15 + 4'd1) > 4'd0` as 1 where the 4-bit
                // sum wraps to 0 and BOTH oracles say 0, and the same 16-vs-0 rode into
                // every generate-if condition and ternary condition built on one.
                // The width-aware walk owns that arm (and consults
                // `const_compare_special` from inside it, so the string / wildcard
                // whole-node folds still fire).
                if !binop_result_is_context_determined(*op) {
                    return self.const_int_selfdet(e);
                }
                let a = self.const_eval_in_scope(lhs)?;
                // The RIGHT operand of `**` and of every shift is a self-determined
                // position (§11.6.1 Table 11-21) — the plain unlimited fold widens it
                // instead. `**` was closed in §4.5.319; the four shifts were not, and
                // once comparisons started redirecting into the width-aware twin (which
                // routes ALL FIVE) the same subexpression answered two different things
                // depending on whether a comparison happened to sit above it:
                // `8'd1 << (4'd15 + 4'd1)` folded 65536 alone and 1 under a `>`.
                // Both oracles say 1 — the 4-bit count wraps to 0.
                if matches!(op, ast::BinOp::Pow) {
                    let (b, sg) = self.const_pow_exponent_selfdet(
                        rhs,
                        &std::collections::BTreeMap::new(),
                        &ConstWidths::new(),
                        0,
                    )?;
                    return const_pow(a, b, sg);
                }
                let b = match op {
                    ast::BinOp::Shl | ast::BinOp::Shr | ast::BinOp::AShl | ast::BinOp::AShr => {
                        self.const_int_selfdet(rhs)?
                    }
                    _ => self.const_eval_in_scope(rhs)?,
                };
                const_binop(*op, a, b)
            }
            // §4.5.186: a call to a CONSTANT FUNCTION in a const context
            // (`localparam W = clog2(N)`). Evaluated by interpreting the function body
            // at compile time (integer domain only; anything it cannot fold → None →
            // LOUD at the binding site, never a silently-wrong param value).
            // `S.len()` parses as a TWO-segment call, so it reaches the const domain
            // here rather than through the `MethodCall` arm above.
            ast::ExprKind::Call { name, .. } if name.segments.len() == 2 => {
                self.const_string_method(e)
            }
            ast::ExprKind::Call { name, args } if name.segments.len() == 1 => self.eval_const_call(
                &name.segments[0].name,
                args,
                &BTreeMap::new(),
                &ConstWidths::new(),
                0,
            ),
            _ => None,
        }
    }

    /// The `**` EXPONENT is a SELF-DETERMINED position (IEEE 1800-2017 §11.6.1
    /// Table 11-21): it is sized and signed by itself, never by the surrounding
    /// context. The plain width-unlimited i64 walk widens it instead — `-4'sd8`
    /// (the 4-bit pattern `1000` = −8) reads back as +8, so
    /// `localparam signed [15:0] P = 4'sd3 ** -4'sd8;` folded 6561 where IEEE
    /// Table 11-6 (negative exponent, |base| > 1) and iverilog answer 0 — while
    /// the RUNTIME lowering of the very same text already answers 0 (§4.5.319
    /// closed the five other spellings; this was the last).
    ///
    /// The self-determined walk (`eval_const_env_self`) DEGRADES to the same
    /// unlimited domain when it cannot size the expression (ctx 0 = no masking)
    /// — deliberately. An earlier draft REFUSED width-unknown wrap-capable
    /// shapes instead, and the adversarial round measured the refusal both ways:
    /// it is only as loud as its CALLER (a range bound's decline path silently
    /// substitutes a default, so a previously-folding `logic [f():0]` became a
    /// 1-bit net at exit 0 = correct→silent-wrong), and it demoted value-exact
    /// cells (`3 ** (m + 0)` over a multi-packed local) from correct to loud.
    /// Degrading keeps every width-unknown cell EXACTLY at its pre-slice
    /// behavior; the shapes that stay imprecise there (a WRAPPING exponent over
    /// a width-unknown leaf — a multi-packed local, a const-array element) are
    /// the interpreter's already-recorded width residual (ROADMAP §2).
    ///
    /// One helper shared by every Pow fold in this domain (`const_eval_in_scope`,
    /// `eval_const_env`, `eval_const_env_at`), so the rule cannot drift between
    /// the module-scope fold and the constant-function interpreter.
    pub(crate) fn const_pow_exponent_selfdet(
        &self,
        rhs: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &ConstWidths,
        depth: u32,
    ) -> Option<(i64, bool)> {
        Some((
            self.eval_const_env_self(rhs, env, envw, depth)?,
            self.const_signed_env(rhs, envw),
        ))
    }

    /// A SELF-DETERMINED integral position OUTSIDE any function body: a `$clog2`
    /// argument, a size cast's SIZE expression, an integral subtree converting to
    /// real (§11.8.1). Such a position has no surrounding integral context — it is
    /// sized and signed by itself — so the plain width-unlimited module-scope fold
    /// widens it instead: `$clog2(4'd15 + 4'd1)` read 16 where the argument is a
    /// 4-bit 0 (iverilog 0 — and vita's OWN constant-function interpreter already
    /// answered 0 for the same text), `(4'd9+4'd8)'(2)` sized 17 bits where the
    /// size is a 4-bit 1, and `2.0 ** -4'sd8` promoted the exponent to +8.
    ///
    /// Same degrade contract as the Pow helper above: where the self width is
    /// unknown the walk keeps the unlimited behavior rather than refusing (the
    /// §4.5.339 measurement — a refusal is only as loud as its CALLER).
    pub(crate) fn const_int_selfdet(&self, e: &ast::Expr) -> Option<i64> {
        self.eval_const_env_self(
            e,
            &std::collections::BTreeMap::new(),
            &ConstWidths::new(),
            0,
        )
    }

    /// `$clog2(arg)` in a const domain — one spelling for all three fold arms
    /// (module scope, plain env twin, width-aware twin).
    ///
    /// The ARGUMENT is a self-determined position (§11.6.1 Table 11-21) and
    /// §20.8.1 says it "shall be treated as an unsigned value" — that is a
    /// reading of the argument's BIT PATTERN at its own width, so
    /// `$clog2(4'sd7 + 4'sd1)` is `$clog2(4'b1000 = 8)` = 3. Verilator and
    /// vita's OWN runtime both answer 3 (iverilog 13.0 answers 32: it converts
    /// the −8 to a 32-bit integer first — recorded as an iverilog divergence,
    /// same family as its `$clog2` self-inconsistencies in §0). The same rule
    /// makes `$clog2(-1)` fold 32 — a 32-bit all-ones pattern — where the old
    /// `n < 0 → None` refusal kept it loud against both oracles.
    ///
    /// Where the argument's width is unknown (0) or beyond the i64 walk (>64),
    /// a non-negative value IS its own unsigned reading; a negative one needs
    /// bits this domain cannot see and stays loud (the pre-slice behavior).
    pub(crate) fn const_clog2_selfdet(
        &self,
        arg: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &ConstWidths,
        depth: u32,
    ) -> Option<i64> {
        let n = match self.const_unsigned_selfdet(arg, env, envw, depth) {
            Some(n) => n,
            // §20.8.1 reads the argument's BIT PATTERN at its own width — and a real
            // has no width to read one out of. What both oracles do instead is the
            // §6.24.1 conversion first and `$clog2` of THAT: `$clog2(2.4)` is 1
            // (2.4 rounds to 2) and `$clog2(0.5)` is 0 (0.5 rounds to 1). A negative
            // result would need the unsigned reading this branch just established it
            // does not have, so it stays loud rather than inventing a width for it.
            // ⚠️ Gated on an EMPTY env for the same reason the walk's catch-all is:
            // `const_int_via_real` resolves names at MODULE scope, so inside a
            // function body — where `env`/`envw` are never empty — it would read a
            // module `real` through an integer local of the same name. That exact
            // shadow was measured on the `$rtoi` sibling and cost a loud →
            // silent-wrong. A real LOCAL is not modelled in this env anyway, so the
            // gate costs nothing it could otherwise have answered.
            None if env.is_empty() && envw.is_empty() => match self.const_int_via_real(arg) {
                Some(v) if v >= 0 => v as u64,
                // …and finally the WIDE bit domain, which is the only one that can read
                // a slice of a >64-bit parameter: `$clog2(M_ISSUE[n*32 +: 32]+1)` is how
                // `axi_crossbar` sizes one port's in-flight counter, and the value lives
                // in `wide_param_bits` where the integer walk cannot see it.
                //
                // ⚠️ `selfdet_bits_unsigned` reads the folded bits back as a u64, so it
                // declines the moment the argument's MAGNITUDE passes 64 bits — and
                // `localparam int AW = $clog2(MAX);` over a 128-bit `MAX` is exactly
                // that shape, the standard width idiom over a crypto constant. The
                // ceiling itself is a BIT INDEX, so the bit domain can answer it
                // without ever forming the value: fall through to `selfdet_clog2_wide`,
                // which returns the finished `$clog2` rather than its argument.
                _ => match self.selfdet_bits_unsigned(arg) {
                    Some(n) => n,
                    None => return self.selfdet_clog2_wide(arg),
                },
            },
            None => return None,
        };
        Some(if n <= 1 {
            0
        } else {
            (64 - (n - 1).leading_zeros()) as i64
        })
    }

    /// The UNSIGNED reading of a SELF-DETERMINED integral position: fold `e` at its
    /// own width, then read the resulting bit pattern as an unsigned number.
    ///
    /// Two positions in the language are spelled exactly this way and this helper is
    /// the single spelling of both, so they cannot drift:
    ///
    ///   * a `$clog2` argument (§20.8.1 "shall be treated as an unsigned value"), and
    ///   * a **delay value** (§7.14 / §28.16) — measured on both oracles:
    ///     `parameter signed [7:0] D = -8'sd1; assign #(D) y = a;` delays **255**
    ///     units (the 8-bit pattern read unsigned), not 1 and not `-1`'s 32-bit
    ///     reading, while `#(4'd15 + 4'd1)` delays **0** because the 4-bit sum wraps.
    ///
    /// Where the self width is unknown (0) or beyond the i64 walk (>64) a
    /// non-negative value IS its own unsigned reading; a negative one would need bits
    /// this domain cannot see, so it declines (the same degrade contract the rest of
    /// the width-aware walk keeps).
    pub(crate) fn const_unsigned_selfdet(
        &self,
        e: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &ConstWidths,
        depth: u32,
    ) -> Option<u64> {
        let v = self.eval_const_env_self(e, env, envw, depth)?;
        match self.const_self_width(e, envw).unwrap_or(0) {
            w @ 1..=63 => Some((v as u64) & ((1u64 << w) - 1)),
            64 => Some(v as u64),
            _ => {
                if v < 0 {
                    return None;
                }
                Some(v as u64)
            }
        }
    }

    /// Fold a SIZE cast `N'(e)` / `W'(e)` — the ONE spelling the module-scope arm
    /// (`const_eval_cast`) and the constant-function arm (`eval_const_env`) share.
    ///
    /// §6.24.1 / §11.8.1: the operand runs at `max(its self width, N)` with the
    /// OPERAND's signedness and the result is delivered in N bits — which is exactly
    /// what the assignment funnel computes for a target of `(N, sign)`, so no second
    /// copy of the width rule exists.
    ///
    /// Declines outside the i64 constant domain, and both halves of that boundary were
    /// measured the hard way:
    ///
    ///   * `N > 64` — the operand still evaluates at 64 bits, so a carry past bit 63 is
    ///     lost. `65'(64'hFFFF_FFFF_FFFF_FFFF + 64'd1) >> 64` is 1 for iverilog AND
    ///     verilator and 0 here, so answering at all is a silent wrong.
    ///   * `N == 64` with an UNSIGNED result whose top bit is set — `coerce_int_width`
    ///     is the identity at 64, so the value escapes as a NEGATIVE i64 and
    ///     `(64'(64'hFFFF_FFFF_FFFF_FFFF) > 0)` answers 0 for iverilog's 1. This is the
    ///     same fit rule `const_placement_env` applies, and the same pre-existing
    ///     64-bit class ROADMAP §2 records for a bare literal — extending it through a
    ///     new syntax is not licensed by its already existing.
    fn const_size_cast(
        &self,
        target: &ast::CastTarget,
        operand: &ast::Expr,
        env: &BTreeMap<String, i64>,
        envw: &ConstWidths,
        depth: u32,
    ) -> Option<i64> {
        let n = u32::try_from(self.cast_size_bits(target)?).ok()?;
        if !(1..=64).contains(&n) {
            return None;
        }
        let sg = self.const_signed_env(operand, envw);
        let v = self.eval_const_assign(operand, env, envw, depth, Some((n, sg)))?;
        if n == 64 && !sg && v < 0 {
            return None;
        }
        Some(v)
    }

    fn const_eval_cast(&self, target: &ast::CastTarget, operand: &ast::Expr) -> Option<i64> {
        match target {
            // `int'(e)`, `byte'(e)`, … — a fixed (width, signedness) target, so the
            // runtime cast's resize-then-sign-stamp is exactly `coerce_int_width`.
            // `real'` yields None from the shared table (no integral value here).
            // ⚠️ Routing this through `const_size_cast` too was tried and measured
            // EQUIVALENT — 13 discriminators (division, modulo and shift under a
            // narrowing `byte'`/`shortint'`, a 64-bit operand, a wrapping sum) all
            // agree, because the final coercion to a FIXED width already reduces
            // whatever the wider evaluation produced. Left alone because a prim cast
            // also changes the 4-state-ness and the domain (`real'` has no integral
            // value at all), and unifying on the strength of "no cell separates them"
            // would be a claim this domain has not earned.
            ast::CastTarget::Prim(p) => {
                let (w, s, _) = cast_prim_wsign(*p)?;
                let v = match self.const_eval_in_scope(operand) {
                    Some(v) => v,
                    // `int'(<real>)` IS the conversion the source asked for, and this
                    // cast node is the context boundary §6.24.1 names — so the operand
                    // folds WHOLE in the real domain and only the rounded result
                    // crosses into the integer one. The integer domain is asked first
                    // and this is a FALLBACK, so a wholly integral operand keeps its
                    // own width semantics (`int'(4'd15 + 4'd1)` stays the 4-bit 0).
                    // `real'` never reaches here: `cast_prim_wsign` has no integral
                    // (width, sign) for it and returned above.
                    None => self.const_int_via_real(operand)?,
                };
                Some(coerce_int_width(v, w, s))
            }
            // `N'(e)`: the operand runs at `max(its self width, N)` and the result is
            // delivered in N bits with the operand's signedness (IEEE §6.24.1 /
            // §11.8.1 — §4.5.316 pinned that rule against iverilog). That is exactly
            // what the assignment funnel computes for a target of `(N, sign)`, and it
            // is the SAME routing `eval_const_env`'s Cast arm already uses, so the
            // two spellings of one construct cannot answer differently.
            //
            // This used to fold the operand in the width-UNLIMITED domain and then
            // truncate, which is unsound on top of an un-narrowed operand
            // (`4'((4'd8+4'd8)/4'd3)` divided 16 by 3 and answered 5 where the 4-bit
            // sum is 0, so SV — and iverilog — answer 0). It compensated by folding
            // only where the signed and unsigned readings AGREE, i.e. a non-negative
            // value with the target's sign bit clear, which declined every ordinary
            // narrowing cast (`8'(255)`, `4'(9)`, `8'(P)`, `64'(-1)`, `1'(3)` — all
            // values iverilog prints). Sizing the operand first removes both: the
            // truncation is no longer an approximation, so the sign no longer has to
            // be guessed away.
            ast::CastTarget::Size(_) | ast::CastTarget::Named(_) => {
                self.const_size_cast(target, operand, &BTreeMap::new(), &ConstWidths::new(), 0)
            }
            // `signed'`/`unsigned'` PRESERVE the operand's width, which this domain
            // does not track (`signed'(4'hF)` is −1 at 4 bits and 15 at 32), so it
            // stays loud.
            ast::CastTarget::Signing { .. } => None,
        }
    }

    /// The bit width a SIZE cast names, for both spellings of one construct.
    ///
    /// The parser makes `4'(e)` a `Size` and `RPS'(e)` a `Named` — a bare identifier
    /// is always `Named`, whether it turns out to be a parameter, a typedef or a
    /// class. Only the parameter reading is a size cast, and the RUNTIME lowering
    /// (`expr_cast`'s `Named` arm) has always resolved it that way; the constant
    /// domain did not, so `localparam logic [RPS-1:0] M = RPS'(1);` lowered fine and
    /// then failed to FOLD — E3009 on a parameter whose value is perfectly known.
    /// One function so the two domains cannot answer differently about the same text.
    pub(crate) fn cast_size_bits(&self, target: &ast::CastTarget) -> Option<i64> {
        match target {
            // The SIZE expression is itself a self-determined position — it has no
            // outer width context — so `(4'd9+4'd8)'(2)` names a 1-bit cast (the
            // 4-bit sum wraps), not a 17-bit one. The unlimited fold answered 17.
            ast::CastTarget::Size(w) => self.const_int_selfdet(w),
            ast::CastTarget::Named(path) if path.segments.len() == 1 => {
                let id = ast::Expr {
                    kind: ast::ExprKind::Ident(path.clone()),
                    span: path.span,
                };
                // Folds ONLY for a genuine constant — a typedef, class or net name
                // yields None, so a real type cast is still not mistaken for a size.
                // Same walk as the Size arm (a bare name cannot wrap, but two walks
                // for one construct is how the spellings drift apart).
                self.const_int_selfdet(&id)
            }
            _ => None,
        }
    }

    /// §4.5.186 constant-function interpreter — the ENV-aware twin of
    /// `const_eval_in_scope`. A single-segment Ident is looked up in the local `env`
    /// (function formals + body locals) BEFORE the module param scope, so a local
    /// shadows a same-named param. Every other form mirrors `const_eval_in_scope`
    /// (sharing `const_binop`). A form not modeled in a const-function body (a
    /// part-select, index, real, string, unmodeled `$call`) returns None → LOUD.
    pub(crate) fn eval_const_env(
        &self,
        e: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &ConstWidths,
        depth: u32,
    ) -> Option<i64> {
        match &e.kind {
            ast::ExprKind::IntLit { .. } => const_eval_i64_lit(e),
            ast::ExprKind::Paren { inner } => self.eval_const_env(inner, env, envw, depth),
            // A single-segment name is this body's own local/formal when it is
            // DECLARED here (`envw` holds every declared name), and only then may
            // it shadow a module param. A name declared but NOT bound is one whose
            // initializer did not fold: reading it is LOUD. Falling through to the
            // module scope there would answer a same-named PARAM for a reference
            // that names the local — a different object than the text says.
            ast::ExprKind::Ident(path) if path.segments.len() == 1 => {
                let n = &path.segments[0].name;
                match env.get(n) {
                    Some(v) => Some(*v),
                    None if envw.contains_key(n) => None,
                    None => self.lookup_scoped(n),
                }
            }
            // `!e` takes a SELF-DETERMINED operand here too — same rule, same reason.
            ast::ExprKind::Unary {
                op: ast::UnOp::LogNot,
                operand,
            } => Some((self.eval_const_env_self(operand, env, envw, depth)? == 0) as i64),
            ast::ExprKind::Unary { op, operand } => {
                let v = self.eval_const_env(operand, env, envw, depth)?;
                match op {
                    ast::UnOp::Plus => Some(v),
                    ast::UnOp::Minus => v.checked_neg(),
                    ast::UnOp::BitNot => Some(!v),
                    _ => None,
                }
            }
            // A non-context-determined operator is a self-determined NODE (the same
            // rule `const_eval_in_scope` and `eval_const_env_at` state); this plain
            // twin would otherwise be the last evaluator in the const domain that
            // contradicts `binop_result_is_context_determined`. Reachable only through
            // `eval_const_assign`'s unknown-target path, so this is a consistency
            // guard rather than a measured defect.
            ast::ExprKind::Binary { op, .. } if !binop_result_is_context_determined(*op) => {
                self.eval_const_env_self(e, env, envw, depth)
            }
            ast::ExprKind::Binary { op, lhs, rhs } => {
                let a = self.eval_const_env(lhs, env, envw, depth)?;
                // `**`'s exponent is SELF-determined — same rule as the
                // width-aware twin's Pow arm (`eval_const_env_at`); this plain
                // walk otherwise widens it. Reachable on its own only through
                // the shape-unknown-target path (`eval_const_assign` with no
                // target), but a second spelling of the rule diverging there
                // would be silent, so it goes through the one shared helper.
                if matches!(op, ast::BinOp::Pow) {
                    let (b, sg) = self.const_pow_exponent_selfdet(rhs, env, envw, depth)?;
                    return const_pow(a, b, sg);
                }
                let b = self.eval_const_env(rhs, env, envw, depth)?;
                const_binop(*op, a, b)
            }
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                if self.eval_const_env_self(cond, env, envw, depth)? != 0 {
                    self.eval_const_env(then_e, env, envw, depth)
                } else {
                    self.eval_const_env(else_e, env, envw, depth)
                }
            }
            ast::ExprKind::SysCall { name, args } if name.name == "$clog2" && args.len() == 1 => {
                // Self-determined argument — same rule as the width-aware twin's
                // arm; this plain walk otherwise widens it (the `**` exponent
                // comment above tells the same story for the same reason).
                self.const_clog2_selfdet(&args[0], env, envw, depth)
            }
            // ⚠️ NO `$rtoi` arm here, deliberately — mirroring the module-scope one
            // was tried and it was a MEASURED silent-wrong: `const_rtoi_via_real`
            // resolves names at module scope, so a body whose integer FORMAL shadows
            // a module `real` of the same name read the parameter instead of the
            // argument (`function int f(int N); f = $rtoi(N);` with `localparam real
            // N = 3.9;` folded `f(9)` to 3 where iverilog gives 9 — and PRE was
            // loud, so that is loud → silent-wrong). Falling through to the catch-all
            // below is not a gap: that arm delegates to the module-scope fold, which
            // HAS the `$rtoi` arm, under a guard that already proves nothing local
            // can be shadowed. The shadow rule belongs to the walk that owns `env`.
            // A nested call's ARGUMENTS are expressions in THIS body, so they must
            // be sized with THIS body's widths — handing over an empty map made
            // `g((b + b) / 8'sd3)` size `b` as 32 bits and compute 66 instead of
            // −18, while the same expression written inline was correct.
            ast::ExprKind::Call { name, args } if name.segments.len() == 1 => {
                self.eval_const_call(&name.segments[0].name, args, env, envw, depth)
            }
            // §11.4.12 concatenation / §11.4.12.1 replication: SELF-determined bit
            // PLACEMENTS whose value depends on every operand's WIDTH, which an i64
            // walk cannot see. The carry-free wide folder already owns that
            // arithmetic (it is what builds a >64-bit parameter), so this borrows it
            // instead of growing a second spelling. Before this arm they fell to the
            // delegation below, which cannot fire inside a body — so a body-local
            // `int x = {4'hA,4'hB}` had no value at all and the declaration bound a
            // silent 0.
            ast::ExprKind::Concat { .. } | ast::ExprKind::Replicate { .. } => {
                self.const_placement_env(e, env, envw)
            }
            // §6.24.1 SIZE cast (`8'(e)`, `RPS'(e)`): §4.5.316 pinned its rule to
            // `max(operand self width, N)` with the operand's sign propagated, and
            // that is exactly what the assignment funnel computes for a target of
            // `(N, sign)` — so the cast routes there rather than carrying a second
            // copy of the width rule.
            // ⚠️ A PRIM cast (`int'(e)`) obeys the SAME width rule — measured:
            // `int'(4'd15 + 4'd1)` is iverilog's 16, i.e. `max(4, 32)` — but it also
            // changes the 4-state-ness and the domain, so routing it here would need
            // its own measurement; a SIGNING cast genuinely does keep its operand
            // self-determined (`signed'(4'd15 + 4'd1)` = 0). Both still decline.
            ast::ExprKind::Cast {
                target: target @ (ast::CastTarget::Size(_) | ast::CastTarget::Named(_)),
                expr,
            } => self.const_size_cast(target, expr, env, envw, depth),
            // A form this env twin does not model itself (`pkg::X`, a const-array
            // element read, `$bits`, a prim/signing cast, …). With NO local
            // bindings there is nothing a module-scope resolution could shadow,
            // so the module-scope fold may answer. Inside a function body this
            // arm can never fire: `eval_const_call` always seeds the
            // function-name return variable, so `env` is never empty there.
            // ⚠️ `depth == 0` does NOT mean "no call is in flight" — a body-local
            // init and `eval_const_env`'s own `Call` arm both fold at the plain
            // caller depth, so depth 0 does occur inside a call (adversarially
            // measured: a probe here fired on three existing suite tests). What
            // makes the delegation terminate is narrower and provable:
            //
            //   * `const_eval_in_scope` RESTARTS the call depth at 0, which
            //     UN-CHARGES a default — and a DEFAULT is the only position that
            //     can re-enter the SAME ast node (it belongs to the callee's own
            //     declaration). So a delegation must never reach a call from
            //     INSIDE one: at depth >= 1 only a CALL-FREE subtree passes.
            //   * At depth 0 a call-bearing subtree is safe because every
            //     recursion from there descends a strictly SMALLER subtree, and
            //     the first default it reaches is charged `depth + 1` — after
            //     which the call-free rule takes over. This is what keeps
            //     `2 ** (8'(cf(2)) + 1)` folding as it always did.
            //   * The call-free half is what lets a plain default like
            //     `input int k = 8'(3)` fold instead of being rejected for the
            //     crash's sake.
            //
            // ⚠️ `envw` is the conjunct with teeth, and it is the ONLY one that
            // carries the argument: `bind_const_decl` records a declaration's width
            // twin BEFORE folding its initializer, so a body expression never sees
            // an empty `envw`, and this arm cannot fire inside a call.
            // ⚠️ The two maps are deliberately NOT lockstep — an earlier note here
            // claimed they were. A declaration whose initializer does not fold is in
            // `envw` and NOT in `env` (that is what makes reading it loud), and,
            // pre-existing, the Blocking lvalue writes `env` with no `envw` twin.
            // Neither breaks this guard, which needs only "envw is non-empty".
            _ if env.is_empty()
                && envw.is_empty()
                && (depth == 0 || !Self::ast_contains_call(e)) =>
            {
                self.const_eval_in_scope(e)
            }
            _ => None,
        }
    }

    /// The carry-free wide fold of a placement expression — a concatenation or a
    /// replication — WITH the width it computed.
    ///
    /// These are self-determined (§11.4.12), and their value depends on every
    /// operand's WIDTH, not just its value, which is why the plain i64 walk cannot do
    /// them. `fold_self_bits` is the carry-free wide folder that already owns exactly
    /// this arithmetic (concat / replication / size cast / constant shift / bitwise),
    /// so this hands it a NAME RESOLVER. Nothing about the placement rules is written
    /// twice.
    ///
    /// The resolver mirrors the Ident arm's rule exactly: a name DECLARED in this body
    /// is the interpreter's own — it shadows a module param, and an UNBOUND one
    /// declines rather than letting a same-named param answer. A name declared with an
    /// unknown shape declines too, because the folder needs a width.
    ///
    /// ⚠️ This returns the RAW fold. Every domain rejection — x/z bits, a result wider
    /// than 64 bits, exactly 64 UNSIGNED bits with the top one set — lives on
    /// [`Self::const_placement_env`], which is the i64 consumer. A caller that wants
    /// only the WIDTH must not inherit those, because a 96-bit concatenation has a
    /// perfectly good width and no i64.
    ///
    /// ⚠️ It also cannot report PROVENANCE. The resolver sizes a name from
    /// `param_meta`, which is where value-INFERRED widths are recorded, and guesses
    /// `(32, false)` when there is none — so a width from here is never
    /// declared-provenance in the `param_decl_width_opt(declared_only)` sense.
    pub(crate) fn const_placement_wide(
        &self,
        e: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &ConstWidths,
    ) -> Option<(ir::BitPacked, u32, bool)> {
        let resolve = |n: &ast::Expr, is_count: bool| -> Option<WideBits> {
            // This resolver answers BARE names only. A `pkg::name` reaching the hook
            // (the shared arm now routes both spellings here) declines, which is what
            // it did before the hook took an expression: the env/`param_meta` pair
            // below is a MODULE-scope lookup and has nothing to say about a package.
            let ast::ExprKind::Ident(path) = &n.kind else {
                return None;
            };
            let [seg] = path.segments.as_slice() else {
                return None; // a hierarchical name is not a constant here
            };
            // ⚠️⚠️ A COUNT / SIZE position may not read this interpreter's LOCALS. They
            // are runtime storage — `int n = 2; {n{4'hA}}` is not a constant expression
            // and iverilog rejects the function — and a local that merely SHADOWS must
            // stop the walk here rather than fall through to the module parameter
            // below, which would answer for a different object than the reference
            // names. Both were measured as blocking defects in §4.5.371.
            if is_count && (envw.contains_key(&seg.name) || env.contains_key(&seg.name)) {
                return None;
            }
            let (v, w, s) = match envw.get(&seg.name).copied() {
                Some((w, s)) if (1..=64).contains(&w) => (*env.get(&seg.name)?, w, s),
                Some(_) => return None, // declared here, shape unknown or too wide
                None => {
                    let v = self.lookup_scoped(&seg.name)?;
                    let (w, s) = self
                        .walk_scopes(&seg.name, &self.param_meta)
                        .unwrap_or((32, false));
                    if !(1..=64).contains(&w) {
                        return None;
                    }
                    (v, w, s)
                }
            };
            let keep = if w >= 64 { !0u64 } else { (1u64 << w) - 1 };
            // One word is exactly `bp_zero(w)`'s allocation for w in 1..=64, and the
            // two planes must stay the same length — `bp_set` bounds-checks `val` and
            // then indexes `unk` with the same word, so an unequal pair would panic.
            let bits = ir::BitPacked {
                val: vec![(v as u64) & keep],
                unk: vec![0],
            };
            debug_assert_eq!(bits.val.len(), bits.unk.len());
            Some((bits, w, s))
        };
        fold_self_bits(e, &resolve)
    }

    /// [`Self::const_placement_wide`] converted back into the i64 constant domain.
    ///
    /// This is where the domain's edges live: it declines for any x/z bit, for a
    /// result wider than 64 bits, and for exactly 64 UNSIGNED bits with the top one
    /// set — the boundary the deleted hand-rolled loop spelled as `total > 63`. It
    /// also inherits every operand shape the carry-free folder refuses, notably
    /// `+`/`-`/`*`/`/`, so `{4'd2, (4'd1 + 4'd1)}` stays loud rather than growing a
    /// second, subtly different, arithmetic here.
    pub(crate) fn const_placement_env(
        &self,
        e: &ast::Expr,
        env: &std::collections::BTreeMap<String, i64>,
        envw: &ConstWidths,
    ) -> Option<i64> {
        let (b, w, sg) = self.const_placement_wide(e, env, envw)?;
        if w > 64 || b.unk.iter().any(|&u| u != 0) {
            return None;
        }
        let v = *b.val.first()?;
        // ⚠️ The i64 const domain carries 63 UNSIGNED magnitude bits. The hand-rolled
        // loop this replaced capped a concatenation at `total > 63`, and that cap was
        // a DOMAIN guard, not a capability limit — dropping it let
        // `{32'hFFFFFFFF, 32'h0}` reach a consumer as a NEGATIVE i64, where `> 0`
        // answered false against BOTH oracles and a generate-if took the other branch,
        // at exit 0. Decline only the values that do not fit: a 64-bit placement whose
        // top bit is CLEAR still folds (`{32'h1, 32'h0}` = 4294967296), which is the
        // fold the old cap gave away. A 64-bit SIGNED result is a bit container by the
        // same convention `const_eval_i64_lit` uses for a 64-bit literal.
        // ⚠️ `!sg` is VACUOUS today and nothing pins it: both callers pass a
        // `Concat`/`Replicate`, and §11.4.12 makes those unsigned, so
        // `fold_self_bits` always answers `sg == false` here. It is kept because the
        // condition is about the DOMAIN, not about this call site — a signed 64-bit
        // result is a bit container by the same convention `const_eval_i64_lit` uses.
        if w == 64 && !sg && v >> 63 != 0 {
            return None;
        }
        Some(coerce_int_width(v as i64, w, sg))
    }

    /// Bind ONE body/block declaration into the interpreter's env pair — the single
    /// spelling both declaration sites share.
    ///
    /// The WIDTH twin is recorded first and unconditionally: a declared name is
    /// visible even when its SHAPE could not be determined (a multi-packed local),
    /// where width 0 records UNKNOWN so no masking is invented for it.
    ///
    /// The VALUE env is populated only when the declaration actually produced one:
    ///
    ///   * no initializer      → 0 (⚠️ pre-existing gap: for a FOUR-state kind IEEE
    ///     says x, and iverilog prints x for `integer x; g = x + 1;` where this
    ///     domain answers 1 — the i64 interpreter carries no unknown, tracked in
    ///     ROADMAP §2),
    ///   * initializer folds   → its value,
    ///   * initializer does NOT fold → the name is left UNBOUND.
    ///
    /// That third case used to share an `unwrap_or(0)` with the first, so
    /// `int x = 8'(5); g = x;` returned 0 where iverilog returns 5 — a silently
    /// wrong parameter at exit 0, and the opposite of what the very same text
    /// written `int x; x = 8'(5);` already did (LOUD, through the assignment arm).
    /// Leaving the name unbound separates them: `eval_const_env`'s Ident arm makes
    /// a READ of an unbound local loud, while a later assignment binds it — so a
    /// local whose unfoldable initializer is DEAD still folds exactly as before
    /// (measured: `int x = 8'(5); x = 42; g = x;` stays 42, iverilog's answer).
    /// `remove` rather than "leave whatever was there" is defensive: the reachable
    /// inner-redeclaration cases are all rejected upstream by the block-local
    /// shadow gate ("`x` is referenced outside its `begin…end` block"), so no
    /// measured design depends on it — but the alternative is a stale outer value
    /// answering for a name whose own initializer failed, which is the silent shape
    /// this whole helper exists to remove.
    ///
    /// `depth` is the BODY's depth — one level already charged for this call — and
    /// that is what bounds a SELF-referential initializer. `int t = g();` inside
    /// `g` re-enters the same ast node, and folding it at the caller's depth never
    /// advanced the cap: it recursed until the stack overflowed. This is the same
    /// position, and the same charge, as a formal's DEFAULT.
    fn bind_const_decl(
        &self,
        d: &ast::NetVarDecl,
        env: &mut std::collections::BTreeMap<String, i64>,
        envw: &mut ConstWidths,
        depth: u32,
    ) -> Option<()> {
        if !netvar_kind_is_int_const(d.kind) {
            return None; // real/string/array local → loud
        }
        let m = self.const_decl_wsign(d.kind, d.range.as_ref(), &d.packed, d.signed);
        for n in &d.names {
            envw.insert(n.name.name.clone(), m.unwrap_or((0, false)));
            match &n.init {
                None => {
                    env.insert(n.name.name.clone(), 0);
                }
                // ⚠️ `m.is_none()` means the DECLARED WIDTH did not fold (its range calls
                // the function being folded, say). A value bound at an unknown width is
                // a value nothing will truncate, so the local stays UNBOUND and every
                // read of it is loud — the property this file pins as "the loud must not
                // be conditional on the width being known". It used to hold only because
                // the initializers that reach such a declaration did not fold either;
                // widening the placement folder made the width the load-bearing half.
                Some(e) if m.is_some() => match self.eval_const_assign(e, env, envw, depth, m) {
                    Some(v) => {
                        env.insert(n.name.name.clone(), v);
                    }
                    None => {
                        env.remove(&n.name.name);
                    }
                },
                Some(_) => {
                    env.remove(&n.name.name);
                }
            }
        }
        Some(())
    }

    /// §4.5.186: evaluate a CONSTANT FUNCTION call `name(args)` by interpreting its
    /// body at compile time. `args` fold in the CALLER's env; a fresh callee env binds
    /// each INPUT formal to its arg value and each body-local to its folded init (see
    /// `bind_const_decl` — an init that does NOT fold leaves the local unbound),
    /// then the body runs. Returns the `return expr` value, else the function-name
    /// return var, coerced to the declared return width. None (→ LOUD, never a wrong
    /// param value) for anything outside the integer domain: a real/string return or
    /// formal/local, an output/inout/ref or unpacked-array formal, an arity mismatch,
    /// an unsupported statement, recursion past the depth cap, or a loop past the step
    /// cap (a guaranteed-terminate guard). The i64 domain matches `const_eval_in_scope`
    /// (its intermediate-width imprecision is the same tracked §2 residual).
    pub(crate) fn eval_const_call(
        &self,
        name: &str,
        args: &[ast::Expr],
        caller_env: &std::collections::BTreeMap<String, i64>,
        caller_w: &ConstWidths,
        depth: u32,
    ) -> Option<i64> {
        const MAX_DEPTH: u32 = 64;
        if depth >= MAX_DEPTH {
            return None;
        }
        let f = self.const_func_table.get(name)?;
        let (rw, rs) = self.const_fn_ret_wsign(f)?;
        if args.len() > f.ports.len() {
            return None; // too many args
        }
        let mut env: std::collections::BTreeMap<String, i64> = std::collections::BTreeMap::new();
        let mut envw: ConstWidths = ConstWidths::new();
        for (i, p) in f.ports.iter().enumerate() {
            if p.dir != ast::PortDir::Input || !p.unpacked.is_empty() {
                return None; // output/inout/ref or unpacked-array formal → loud
            }
            if let Some(k) = p.net_or_var {
                if !netvar_kind_is_int_const(k) {
                    return None; // real/string/… formal → loud
                }
            }
            // §11.6: an argument is assigned to the formal, so it takes the
            // FORMAL's declared width — a `bit [3:0]` formal receiving `4'd15+1`
            // gets 0, not 16.
            // A tf-port with no data type is `logic` with the given range (1 bit
            // when there is none) — `input [3:0] a` is the commonest Verilog-2005
            // spelling and used to get no width at all.
            let tw = self.const_decl_wsign(
                p.net_or_var.unwrap_or(ast::NetVarKind::Logic),
                p.range.as_ref(),
                &[],
                p.signed,
            );
            // An explicit ARGUMENT folds at the CALLER's depth: it descends a
            // finite AST (`g(g(g(0)))` is three distinct nodes), so charging it
            // a level only shrinks how deep a legitimate design may nest —
            // measured: 65-deep argument nesting folds on the pre-slice binary
            // and on iverilog, and went LOUD when args were charged.
            //
            // A DEFAULT is the cyclic one: it belongs to the CALLEE's own
            // declaration, so folding `input int k = f()` re-enters the SAME
            // node and recursed at constant depth until the stack overflowed
            // (pre-existing; the `8'(f())` spelling reached it through the
            // module-scope delegation below). One level per default makes the
            // depth cap bound it — E3009, not a crash. iverilog aborts on both.
            let av = if let Some(a) = args.get(i) {
                self.eval_const_assign(a, caller_env, caller_w, depth, tw)?
            } else if let Some(d) = &p.default {
                self.eval_const_assign(d, &env, &envw, depth + 1, tw)?
            } else {
                return None; // too few args, no default
            };
            // Record the shape — or UNKNOWN (width 0) when it could not be
            // determined, so a later reader propagates "no masking" instead of
            // falling back to a guessed 32-bit unsigned.
            envw.insert(p.name.name.clone(), tw.unwrap_or((0, false)));
            env.insert(p.name.name.clone(), av);
        }
        // The body's declarations run at the BODY's depth — the same `depth + 1`
        // the body itself gets below, and the reason a self-referential
        // initializer terminates. See `bind_const_decl`.
        for d in &f.body_decls {
            self.bind_const_decl(d, &mut env, &mut envw, depth + 1)?;
        }
        // The function-name return variable is itself a declared target.
        envw.insert(name.to_string(), (rw, rs));
        env.entry(name.to_string()).or_insert(0);
        let mut steps: u64 = 0;
        let ret = match self.exec_const_stmt(
            &f.body,
            &mut env,
            &mut envw,
            Some((rw, rs)),
            depth + 1,
            &mut steps,
        )? {
            ConstFlow::Return(Some(v)) => v,
            ConstFlow::Return(None) | ConstFlow::Normal => *env.get(name)?,
        };
        Some(coerce_int_width(ret, rw, rs))
    }

    /// §4.5.186: execute one statement of a const-function body over the local `env`.
    /// Supports the pure integer subset — Block (+ local decls), blocking `=`, if/else,
    /// for/while/repeat, return. A NonBlocking/timing/fork/system-task/case or any
    /// unmodeled form returns None → LOUD. `steps` bounds total iterations so a
    /// non-terminating loop is loud, never a hang.
    pub(crate) fn exec_const_stmt(
        &self,
        s: &ast::Stmt,
        env: &mut std::collections::BTreeMap<String, i64>,
        envw: &mut ConstWidths,
        ret: Option<(u32, bool)>,
        depth: u32,
        steps: &mut u64,
    ) -> Option<ConstFlow> {
        // A generous bound (legit const functions loop a few times — clog2 ~64,
        // factorial ~20); a runaway/non-terminating loop trips it and goes LOUD
        // rather than hanging elaboration. Kept modest so the loud is prompt even in
        // an unoptimized (test) build.
        const MAX_STEPS: u64 = 100_000;
        *steps += 1;
        if *steps > MAX_STEPS {
            return None;
        }
        match s {
            ast::Stmt::Null(_) => Some(ConstFlow::Normal),
            ast::Stmt::Block { decls, stmts, .. } => {
                // `depth` here is already the body's depth (`eval_const_call`
                // charged the call), so it is the same value the function-level
                // declarations get.
                for d in decls {
                    self.bind_const_decl(d, env, envw, depth)?;
                }
                for st in stmts {
                    match self.exec_const_stmt(st, env, envw, ret, depth, steps)? {
                        ConstFlow::Normal => {}
                        other => return Some(other),
                    }
                }
                Some(ConstFlow::Normal)
            }
            ast::Stmt::Blocking {
                lhs,
                delay,
                event,
                rhs,
                ..
            } => {
                if delay.is_some() || event.is_some() {
                    return None; // intra-assignment timing → loud
                }
                let ast::Lvalue::Ident(path) = lhs else {
                    return None; // only a simple local-var target
                };
                if path.segments.len() != 1 {
                    return None;
                }
                // §11.6: the RHS runs at max(its self width, the TARGET's width)
                // and is then assigned — this is where `bit [3:0] t = 4'd15+4'd15`
                // becomes 14 instead of 30.
                let tw = envw.get(&path.segments[0].name).copied();
                let v = self.eval_const_assign(rhs, env, envw, depth, tw)?;
                env.insert(path.segments[0].name.clone(), v);
                Some(ConstFlow::Normal)
            }
            ast::Stmt::If {
                cond,
                then_s,
                else_s,
                ..
            } => {
                if self.eval_const_env_self(cond, env, envw, depth)? != 0 {
                    self.exec_const_stmt(then_s, env, envw, ret, depth, steps)
                } else if let Some(e) = else_s {
                    self.exec_const_stmt(e, env, envw, ret, depth, steps)
                } else {
                    Some(ConstFlow::Normal)
                }
            }
            ast::Stmt::Return { value, .. } => {
                // `return e` assigns the function's declared return type; the
                // caller coerces again, which is idempotent.
                let v = match value {
                    // `return e` assigns the function's DECLARED return type, so it
                    // takes the same context width as `f = e` — without this the two
                    // spellings of one expression disagreed (`return (4'd15+4'd15)>>1`
                    // gave 15 where the assignment form gave iverilog's 7).
                    Some(e) => Some(self.eval_const_assign(e, env, envw, depth, ret)?),
                    None => None,
                };
                Some(ConstFlow::Return(v))
            }
            ast::Stmt::While { cond, body, .. } => {
                while self.eval_const_env_self(cond, env, envw, depth)? != 0 {
                    match self.exec_const_stmt(body, env, envw, ret, depth, steps)? {
                        ConstFlow::Normal => {}
                        other => return Some(other),
                    }
                }
                Some(ConstFlow::Normal)
            }
            ast::Stmt::For {
                init,
                cond,
                step,
                body,
                ..
            } => {
                self.exec_const_stmt(init, env, envw, ret, depth, steps)?;
                while self.eval_const_env_self(cond, env, envw, depth)? != 0 {
                    match self.exec_const_stmt(body, env, envw, ret, depth, steps)? {
                        ConstFlow::Normal => {}
                        other => return Some(other),
                    }
                    self.exec_const_stmt(step, env, envw, ret, depth, steps)?;
                }
                Some(ConstFlow::Normal)
            }
            ast::Stmt::Repeat { count, body, .. } => {
                let n = self.eval_const_env_self(count, env, envw, depth)?;
                if n < 0 {
                    return None;
                }
                for _ in 0..n {
                    match self.exec_const_stmt(body, env, envw, ret, depth, steps)? {
                        ConstFlow::Normal => {}
                        other => return Some(other),
                    }
                }
                Some(ConstFlow::Normal)
            }
            // NonBlocking / timing / fork / system-task / case / disable / … → loud.
            _ => None,
        }
    }
}
