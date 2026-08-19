//! constant-function interpreter — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// Value AND width of a constant sub-expression, for the width-carrying folds
/// (concatenation) where an `i64` alone loses the information that decides the
/// result. `None` whenever the width is not statically determinable — the caller
/// then stays loud rather than guessing a width, which would silently change the
/// value (a concat's result depends on every operand's width, not just its value).
fn const_eval_sized(e: &ast::Expr) -> Option<(i64, u32)> {
    match &e.kind {
        ast::ExprKind::Paren { inner } => const_eval_sized(inner),
        ast::ExprKind::IntLit { kind, raw } => {
            let cv = literal::parse_int_literal(raw, *kind)?;
            Some((const_eval_i64_lit(e)?, cv.width))
        }
        _ => None,
    }
}

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
pub(crate) fn const_pow_i64(a: i64, b: i64) -> Option<i64> {
    if b < 0 {
        return match a {
            1 => Some(1),
            -1 => Some(if b % 2 == 0 { 1 } else { -1 }),
            0 => None,
            _ => Some(0),
        };
    }
    a.checked_pow(u32::try_from(b).ok()?)
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
        ast::BinOp::Pow => const_pow_i64(a, b),
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
            // take the low 32 bits of the value plane (2-state by the check above).
            Some(cv.bits.val.first().copied().unwrap_or(0) as u32)
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
            // §11.4.12 concatenation of constants: `{4'b0001, 32'b0}`. Each operand is
            // SELF-DETERMINED, so the result is the operands' bits laid end to end and
            // its value depends on every operand's WIDTH — which is why this needs
            // `const_eval_sized_in_scope` and not the plain i64 fold.
            //
            // Folds only while every operand's width is known and the total stays inside
            // the i64 const domain; anything else returns None and the caller reports its
            // usual "not a foldable constant expression" rather than inventing a value.
            //
            // Found by elaborating PicoRV32, whose trace-mask localparams are written
            // `localparam [35:0] TRACE_BRANCH = {4'b 0001, 32'b 0};`.
            ast::ExprKind::Concat { parts } => {
                let mut acc: i64 = 0;
                let mut total: u32 = 0;
                for p in parts {
                    let (v, w) = const_eval_sized(p)?;
                    if w == 0 || total.checked_add(w)? > 63 {
                        return None; // outside the i64 const domain — stay loud
                    }
                    let masked = if w >= 64 { v } else { v & ((1i64 << w) - 1) };
                    acc = acc.checked_shl(w)? | masked;
                    total += w;
                }
                Some(acc)
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
            ast::ExprKind::Unary { op, operand } => {
                let v = self.const_eval_in_scope(operand)?;
                match op {
                    ast::UnOp::Plus => Some(v),
                    ast::UnOp::Minus => v.checked_neg(),
                    ast::UnOp::BitNot => Some(!v),
                    ast::UnOp::LogNot => Some((v == 0) as i64),
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
                    return None;
                }
                self.const_array_vals_of_base(base)?
                    .get(idx as usize)
                    .copied()
            }
            ast::ExprKind::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                let c = self.const_eval_in_scope(cond)?;
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
                self.bits_of_view(&args[0], true).map(|n| n as i64)
            }
            // A static cast in a constant context (`int'(7)`, `8'(P+1)`). Without
            // this arm `int'(7)` was NOT a foldable constant, so every bound/count
            // site fell back to a weaker folder and degraded SILENTLY (a part-select
            // width collapsed to 1, a replication count to 0).
            ast::ExprKind::Cast { target, expr } => self.const_eval_cast(target, expr),
            ast::ExprKind::Binary { op, lhs, rhs } => {
                // A STRING comparison folds in the string domain, before the i64 one —
                // `MODE == "Y"` on a `parameter string MODE` is the canonical way to
                // switch an implementation in a generate-if, and neither operand has an
                // i64 value, so the numeric fold below returned None and the generate-if
                // was loud ("condition is not a constant"). IEEE 1800 §6.16 compares
                // string VALUES, so equality is exact text equality.
                if matches!(
                    op,
                    ast::BinOp::Eq | ast::BinOp::Ne | ast::BinOp::CaseEq | ast::BinOp::CaseNe
                ) {
                    if let (Some(x), Some(y)) =
                        (self.const_str_in_scope(lhs), self.const_str_in_scope(rhs))
                    {
                        let want_eq = matches!(op, ast::BinOp::Eq | ast::BinOp::CaseEq);
                        return Some(i64::from((x == y) == want_eq));
                    }
                }
                let a = self.const_eval_in_scope(lhs)?;
                // `==?`/`!=?` against a wildcard LITERAL (`P ==? 4'b1x1x`): the x/z bits
                // of the PATTERN (rhs) are don't-cares (§11.4.6). const_eval carries no
                // x/z, so the generic `const_eval_in_scope(rhs)` below returns None on
                // the pattern; pull the pattern's value + x/z mask straight from the
                // literal and masked-compare. The pattern zero-extends, so `a & !mask`
                // at full width matches iverilog for a narrower pattern too. Fail-closed:
                // only a NON-NEGATIVE const `a` (an i64 sign bit would corrupt the
                // full-width compare) and a single-word, bit-63-clear pattern; otherwise
                // fall through to None (loud). An x/z-free pattern is NOT intercepted —
                // it folds via the `WildEq`/`WildNe` collapse arm below.
                if matches!(op, ast::BinOp::WildEq | ast::BinOp::WildNe) {
                    if let ast::ExprKind::IntLit { kind, raw } = &rhs.kind {
                        // Only a SIZED pattern is safe: bits ABOVE its declared width
                        // zero-extend, so the masked compare's "the LHS high bits must
                        // be 0" is correct. An UNSIZED x/z literal (`'hx`) x-FILLS to the
                        // context width — but parse_int_literal sizes it to its 32-bit
                        // self-width, so an LHS wider than 32 bits would wrongly require
                        // its high bits to be 0 (silent-wrong). Leave unsized x/z patterns
                        // loud (fall through → the generic rhs fold returns None).
                        if matches!(kind, ast::IntLitKind::Sized) {
                            if let Some(cv) = parse_int_literal(raw, *kind) {
                                if cv.bits.unk.iter().any(|&u| u != 0) {
                                    let pat = cv.bits.val.first().copied().unwrap_or(0);
                                    let mask = cv.bits.unk.first().copied().unwrap_or(0);
                                    if a >= 0
                                        && cv.bits.val.len() <= 1
                                        && cv.bits.unk.len() <= 1
                                        && (pat >> 63) == 0
                                        && (mask >> 63) == 0
                                    {
                                        let eq =
                                            (a & !(mask as i64)) == (pat as i64 & !(mask as i64));
                                        return Some(if matches!(op, ast::BinOp::WildEq) {
                                            eq
                                        } else {
                                            !eq
                                        }
                                            as i64);
                                    }
                                    return None; // negative LHS / wide / bit-63 pattern → loud
                                }
                            }
                        }
                    }
                }
                let b = if matches!(op, ast::BinOp::Pow) {
                    // `**`'s EXPONENT is self-determined (§11.6.1 Table 11-21) —
                    // the plain unlimited fold below widens it instead, which is
                    // exactly the §4.5.319 defect this domain still had.
                    self.const_pow_exponent_selfdet(
                        rhs,
                        &std::collections::BTreeMap::new(),
                        &ConstWidths::new(),
                        0,
                    )?
                } else {
                    self.const_eval_in_scope(rhs)?
                };
                const_binop(*op, a, b)
            }
            // §4.5.186: a call to a CONSTANT FUNCTION in a const context
            // (`localparam W = clog2(N)`). Evaluated by interpreting the function body
            // at compile time (integer domain only; anything it cannot fold → None →
            // LOUD at the binding site, never a silently-wrong param value).
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
    ) -> Option<i64> {
        self.eval_const_env_self(rhs, env, envw, depth)
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
        let v = self.eval_const_env_self(arg, env, envw, depth)?;
        let n: u64 = match self.const_self_width(arg, envw).unwrap_or(0) {
            w @ 1..=63 => (v as u64) & ((1u64 << w) - 1),
            64 => v as u64,
            _ => {
                if v < 0 {
                    return None;
                }
                v as u64
            }
        };
        Some(if n <= 1 {
            0
        } else {
            (64 - (n - 1).leading_zeros()) as i64
        })
    }

    /// A static cast `casting_type'(e)` in the INTEGER const domain (the
    /// `const_eval_in_scope` arm). Correct-or-loud: only the forms whose value is
    /// exact without tracking an operand WIDTH fold; everything else is None ⇒ the
    /// caller stays loud rather than binding a reinterpreted value.
    /// The STRING value of a constant expression, or None.
    ///
    /// Deliberately narrow: a string literal, a parenthesised one, a `pkg::NAME`, or a
    /// bare name bound in `str_param_raw` — resolved through the SAME innermost-wins
    /// `walk_scopes_key` over the SAME combined binding set that the lowering path
    /// uses (`expr_main`'s Ident arm), so a local net or an integer param that shadows
    /// a string param wins here exactly as it does there. Two spellings of that rule
    /// is how a generate-if would decide one way and the body lower the other.
    pub(crate) fn const_str_in_scope(&self, e: &ast::Expr) -> Option<String> {
        match &e.kind {
            ast::ExprKind::StrLit { raw } => Some(raw.clone()),
            ast::ExprKind::Paren { inner } => self.const_str_in_scope(inner),
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                let seg = &p.segments[0].name;
                let key = self.walk_scopes_key(seg, |k| {
                    self.str_param_raw.contains_key(k)
                        || self.real_param_val.contains_key(k)
                        || self.params.contains_key(k)
                        || self.symbols.contains_key(k)
                })?;
                self.str_param_raw.get(&key).cloned()
            }
            ast::ExprKind::PkgScoped { pkg, name } => self
                .str_param_raw
                .get(&format!("{}::{}", pkg.name, name.name))
                .cloned(),
            _ => None,
        }
    }

    fn const_eval_cast(&self, target: &ast::CastTarget, operand: &ast::Expr) -> Option<i64> {
        let v = self.const_eval_in_scope(operand)?;
        match target {
            // `int'(e)`, `byte'(e)`, … — a fixed (width, signedness) target, so the
            // runtime cast's resize-then-sign-stamp is exactly `coerce_int_width`.
            // `real'` yields None from the shared table (no integral value here).
            ast::CastTarget::Prim(p) => {
                let (w, s, _) = cast_prim_wsign(*p)?;
                Some(coerce_int_width(v, w, s))
            }
            // `N'(e)`: N bits, signedness INHERITED from the operand — which this
            // domain does not track. Fold only where both interpretations agree: a
            // non-negative value that leaves the target's sign bit clear. (`4'(9)`
            // is 9 unsigned but −7 signed, so it stays loud.)
            ast::CastTarget::Size(_) | ast::CastTarget::Named(_) => {
                let n = self.cast_size_bits(target)?;
                if !(1..=64).contains(&n) || v < 0 {
                    return None;
                }
                // At 64 bits every non-negative i64 is already representable with the
                // sign bit clear; below that the check must actually run (at exactly
                // 63 it is `v < 2^62`, NOT a bypass — bypassing it would return a
                // positive value the 63-bit signed reading calls negative).
                if n >= 64 {
                    return Some(v);
                }
                (v < (1i64 << (n - 1))).then_some(v)
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
            ast::ExprKind::Ident(path) if path.segments.len() == 1 => env
                .get(&path.segments[0].name)
                .copied()
                .or_else(|| self.lookup_scoped(&path.segments[0].name)),
            ast::ExprKind::Unary { op, operand } => {
                let v = self.eval_const_env(operand, env, envw, depth)?;
                match op {
                    ast::UnOp::Plus => Some(v),
                    ast::UnOp::Minus => v.checked_neg(),
                    ast::UnOp::BitNot => Some(!v),
                    ast::UnOp::LogNot => Some((v == 0) as i64),
                    _ => None,
                }
            }
            ast::ExprKind::Binary { op, lhs, rhs } => {
                let a = self.eval_const_env(lhs, env, envw, depth)?;
                // `**`'s exponent is SELF-determined — same rule as the
                // width-aware twin's Pow arm (`eval_const_env_at`); this plain
                // walk otherwise widens it. Reachable on its own only through
                // the shape-unknown-target path (`eval_const_assign` with no
                // target), but a second spelling of the rule diverging there
                // would be silent, so it goes through the one shared helper.
                let b = if matches!(op, ast::BinOp::Pow) {
                    self.const_pow_exponent_selfdet(rhs, env, envw, depth)?
                } else {
                    self.eval_const_env(rhs, env, envw, depth)?
                };
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
            // A nested call's ARGUMENTS are expressions in THIS body, so they must
            // be sized with THIS body's widths — handing over an empty map made
            // `g((b + b) / 8'sd3)` size `b` as 32 bits and compute 66 instead of
            // −18, while the same expression written inline was correct.
            ast::ExprKind::Call { name, args } if name.segments.len() == 1 => {
                self.eval_const_call(&name.segments[0].name, args, env, envw, depth)
            }
            // A form this env twin does not model itself (`pkg::X`, a concat, a
            // const-array element read, `$bits`, a cast, …). With NO local
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
            // ⚠️ `envw` is the conjunct with teeth: a formal/local is recorded
            // there BEFORE its init folds, so a body expression never sees an
            // empty map. `env` is its lockstep twin (measured: a probe found no
            // state where the two disagree, 0 hits in 247 tests) and is asserted
            // with it so a future path that populates only one cannot pass.
            _ if env.is_empty()
                && envw.is_empty()
                && (depth == 0 || !Self::ast_contains_call(e)) =>
            {
                self.const_eval_in_scope(e)
            }
            _ => None,
        }
    }

    /// §4.5.186: evaluate a CONSTANT FUNCTION call `name(args)` by interpreting its
    /// body at compile time. `args` fold in the CALLER's env; a fresh callee env binds
    /// each INPUT formal to its arg value and each body-local to its folded init (or 0),
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
        for d in &f.body_decls {
            if !netvar_kind_is_int_const(d.kind) {
                return None; // real/string/array local → loud
            }
            let m = self.const_decl_wsign(d.kind, d.range.as_ref(), &d.packed, d.signed);
            for n in &d.names {
                envw.insert(n.name.name.clone(), m.unwrap_or((0, false)));
                let iv = n
                    .init
                    .as_ref()
                    .and_then(|e| self.eval_const_assign(e, &env, &envw, depth, m))
                    .unwrap_or(0);
                env.insert(n.name.name.clone(), iv);
            }
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
                for d in decls {
                    if !netvar_kind_is_int_const(d.kind) {
                        return None;
                    }
                    let m = self.const_decl_wsign(d.kind, d.range.as_ref(), &d.packed, d.signed);
                    for n in &d.names {
                        envw.insert(n.name.name.clone(), m.unwrap_or((0, false)));
                        let iv = n
                            .init
                            .as_ref()
                            .and_then(|e| self.eval_const_assign(e, env, envw, depth, m))
                            .unwrap_or(0);
                        env.insert(n.name.name.clone(), iv);
                    }
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
