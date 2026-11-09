//! const_fold -- the parser's parse-time constant fold (ROADMAP §5.2 PARSE-1, ER §5.1).
//!
//! History (owner 2026-11-09): every fn that folds an AST expression or a literal to a value at
//! parse time was collected here from `expr_primary.rs`, `packed_md.rs`,
//! `structs/member_array.rs`, `structs/mod.rs` and `params.rs`, beside the tables it reads
//! (`const_locals`, `pkg_const_scoped`): "an evaluator stays where its state is". The 64-bit
//! kernels it shares with elaborate (the op sets over `const_binop`, the context-width arm,
//! `$clog2`, the decimal read, the sign extension) live in hdl-ast `value_rules.rs`.
//! STRUCTURE ONLY: each body moved verbatim (methods of `Parser` keep their call text), except
//! the one-expression re-points to those kernels; each caller keeps its own operand order, its
//! `$clog2` `n < 0` check and its cutoffs.
//!
//! Lanes (named answers, never merged): L2 `try_const_index` = Parser4 ; L3 `const_lit` = Lit3
//! (no Paren, no names, no based leaf) ; L4 `cw_shape` / `cw_eval` = the context width, masked
//! once here ; L5 / L6 the two `$clog2` readers ; L9 the decimal leaves, `cw_shape`'s i32 and
//! the size reads' u32 cutoffs ; L10 the literal readers kept apart (`based_lit_pattern`,
//! `small_unsized_lit`) ; L11 `const_lit_enum` reads the BASE type's sign ; L12 `read_pattern`'s
//! 64-bit bit-container read.
//!
//! OUTSIDE by rule: O5 other grammars, O6 the unary arms (one std call per fold), O7 the
//! respell builders and the table producer / maintenance (`param_item_to_module_item`, imports,
//! generate, scope, functask, decls, module items), O8 the fold's readers (each decides what a
//! fold's "no" means at its site).
//!
//! ENFORCED by elaborate/tests/value_rules_home.rs: T5 (the 21 folds are defined, within
//! hdl-parser, only here) and T6 (a value read of `const_locals` / `pkg_const_scoped` only
//! here, two named exceptions).

use super::*;

impl Parser<'_, '_> {
    /// Const-fold a generate-array hier index to its non-negative value: a decimal
    /// literal, literal arithmetic (`P-1`, `1+2`), or a module-scope literal-valued
    /// `localparam` (recorded in `const_locals`). A `parameter` / param-derived value
    /// is NOT in `const_locals`, so it does not fold (its index stays loud — folding
    /// it could disagree with an instance override). Returns `None` if not foldable.
    pub(crate) fn try_const_index(&self, e: &Expr) -> Option<i64> {
        match &e.kind {
            ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw,
            } => hdl_ast::value_rules::decimal_i64(raw),
            ExprKind::Ident(p) if p.segments.len() == 1 => {
                self.const_locals.get(&p.segments[0].name).map(|c| c.v)
            }
            // §3 ⑤ ⓓ: a scoped constant `pkg::W` (exported at `endpackage`).
            ExprKind::PkgScoped { pkg, name } => self
                .pkg_const_scoped
                .get(&format!("{}::{}", pkg.name, name.name))
                .map(|c| c.v),
            // §3 ⑤ ⓓ: `$clog2(n)` of a folded non-negative operand (IEEE §20.8.1:
            // the number of bits to address `n` items; 0 for 0 and 1). The same
            // function elaborate folds, over the same integer domain.
            ExprKind::SysCall { name, args } if name.name == "$clog2" && args.len() == 1 => {
                let n = self.try_const_index(&args[0])?;
                if n < 0 {
                    return None;
                }
                Some(i64::from(hdl_ast::value_rules::clog2_u64(n as u64)))
            }
            ExprKind::Paren { inner } => self.try_const_index(inner),
            ExprKind::Unary {
                op: UnOp::Minus,
                operand,
                // (`checked_neg` only differs at `i64::MIN`, which no decimal literal
                // can reach here — `9223372036854775808` does not parse as an i64, so
                // the operand declines first. Kept because the guard is the rule.)
            } => self.try_const_index(operand)?.checked_neg(),
            // ⚠️ CHECKED, because this helper runs on EVERY binary expression the
            // parser sees — not only on the generate-array indices it exists for.
            // Unchecked arithmetic here panicked the whole run on an ordinary
            // `localparam integer L = 3037000500 * 3037000500;`, which both oracles
            // simply fold. A panic is below loud; declining just means "not a
            // constant index", which is what a non-index expression is anyway.
            // ⚠️ The rule was already written down one function away — `const_lit`
            // in this same file folds checked and says why. This was its sibling
            // spelling, and only one of the two had it.
            ExprKind::Binary { op, lhs, rhs } => {
                let a = self.try_const_index(lhs)?;
                let b = self.try_const_index(rhs)?;
                // §3 ⑤ ⓓ: integer division truncates toward zero in both SV
                // (§11.4.3) and Rust; a zero divisor declines (elaborate gives x).
                hdl_ast::value_rules::binop_in(hdl_ast::value_rules::OpSet::Parser4, *op, a, b)
            }
            _ => None,
        }
    }

    /// Fold a constant-literal expression to `i64` at parse time (decimal literals
    /// and +/-/* of them). Returns `None` for anything non-constant.
    /// §4.5.418: the WIDTH-AWARE twin of `try_const_index`, for a constant bound
    /// that a layout is built from (a packed-struct member width). IEEE §11.6: a
    /// decimal literal is 32-bit signed, a based literal its own size (unsized ⇒ 32,
    /// unsigned unless `s`), a named constant its declaration's width and sign
    /// (`ConstVal`), and `+ - * / %` are computed at the LARGER operand width, signed
    /// only when both operands are; the result wraps to that width. `$clog2` yields
    /// a 32-bit signed value of a self-determined operand; unary minus and parens
    /// keep the operand's width. Every other shape, a constant of unknown width
    /// (`w == 0`), a literal that does not fit its width, or a division by zero
    /// declines — the caller then falls back to the i64 fold, byte-identical to
    /// before. Measured need: `int unsigned W = 32'hffff_ffff; [$clog2(W+2):0]` and
    /// `logic [3:0] C = 15, D = 1; [C+D:0]` wrap to 1 in both oracles, where the
    /// i64 fold answered 4294967297 and 17 (review B F1 and the census control twin).
    pub(crate) fn try_const_index_w(&self, e: &Expr) -> Option<ConstVal> {
        self.try_const_index_wc(e, 0)
    }

    /// [`Self::try_const_index_w`] under a CONTEXT width `ctx` (`0` = none) — the
    /// §11.6.1 / §11.8.2 rule, in the LRM's two steps. First the expression's width
    /// and sign are determined from ALL of its context-determined operands and the
    /// context (`cw_shape`): the larger of the operands' widths and `ctx`; signed only
    /// when every operand is. Then every context-determined node is evaluated AT that
    /// width (`cw_eval`), the operands extended first. A leaf keeps its own width for
    /// the first step and is extended in the second; a `$clog2` operand is
    /// self-determined (no context, its own two steps).
    ///
    /// Measured against both oracles: `localparam int E = C + D` over two 4-bit
    /// constants is 16 (the context carries the carry), `logic [3:0] F = C + D` is 0,
    /// `int S = (D - C) / 2` is 2147483641 (unsigned, §11.8.1); in a self-determined
    /// position `g[-C+16]` is `g[1]` and `[(C+D)%3:0]` is `[1:0]` — the first draft
    /// folded bottom-up, wrapping `-C` and `C+D` at 4 bits before the 32-bit literal
    /// beside them widened the expression, and got both wrong (census, 2 cells).
    pub(crate) fn try_const_index_wc(&self, e: &Expr, ctx: u32) -> Option<ConstVal> {
        let (w0, signed) = self.cw_shape(e)?;
        let w = w0.max(ctx);
        if w == 0 || w > 64 {
            return None;
        }
        let pat = self.cw_eval(e, w, signed)?;
        let v = hdl_ast::value_rules::wrap_i128(pat, w, signed)?;
        Some(ConstVal { v, w, signed })
    }

    /// Step one of [`Self::try_const_index_wc`]: the expression's own `(width, signed)`
    /// (§11.6.1 Table 11-21 for the width, §11.8.1 for the sign). `None` = a shape this
    /// fold does not know, or a name whose width is unknown.
    fn cw_shape(&self, e: &Expr) -> Option<(u32, bool)> {
        match &e.kind {
            ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw,
            } => {
                let v = hdl_ast::value_rules::decimal_i64(raw)?;
                i32::try_from(v).ok()?;
                Some((32, true))
            }
            ExprKind::IntLit {
                kind: kind @ (IntLitKind::Sized | IntLitKind::UnsizedBased),
                raw,
            } => {
                let sized = matches!(kind, IntLitKind::Sized);
                let (_, w) = Self::based_lit_pattern(raw, sized)?;
                let tick = raw.find('\'')?;
                let signed = matches!(raw[tick + 1..].chars().next(), Some('s') | Some('S'));
                Some((w, signed))
            }
            ExprKind::Ident(p) if p.segments.len() == 1 => self
                .const_locals
                .get(&p.segments[0].name)
                .filter(|c| c.w > 0)
                .map(|c| (c.w, c.signed)),
            ExprKind::PkgScoped { pkg, name } => self
                .pkg_const_scoped
                .get(&format!("{}::{}", pkg.name, name.name))
                .filter(|c| c.w > 0)
                .map(|c| (c.w, c.signed)),
            ExprKind::Paren { inner } => self.cw_shape(inner),
            ExprKind::Unary {
                op: UnOp::Minus,
                operand,
            } => self.cw_shape(operand),
            ExprKind::SysCall { name, args } if name.name == "$clog2" && args.len() == 1 => {
                Some((32, true))
            }
            ExprKind::Binary { op, lhs, rhs } => {
                if !matches!(
                    op,
                    BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
                ) {
                    return None;
                }
                let (lw, ls) = self.cw_shape(lhs)?;
                let (rw, rs) = self.cw_shape(rhs)?;
                Some((lw.max(rw), ls && rs))
            }
            _ => None,
        }
    }

    /// Step two of [`Self::try_const_index_wc`]: the value of `e` as a `w`-bit pattern
    /// (a non-negative `i128` below 2^w), every context-determined node computed at
    /// `w`, leaves extended per `signed` (the EXPRESSION's sign, §11.8.2 — an unsigned
    /// expression zero-extends a signed leaf). Division and modulus read the operands
    /// under `signed` and truncate toward zero (§11.4.3); a zero divisor declines.
    fn cw_eval(&self, e: &Expr, w: u32, signed: bool) -> Option<i128> {
        let mask: i128 = (1i128 << w) - 1;
        // A leaf's `(value, own width, own sign)` → its pattern at `w`.
        let leaf = |v: i64, lw: u32, ls: bool| -> Option<i128> {
            let x: i128 = if signed && ls {
                v as i128
            } else {
                // Zero-extend the leaf's own `lw`-bit pattern.
                (hdl_ast::value_rules::wrap_i128(v as i128, lw, false)? as u64) as i128
            };
            Some(x & mask)
        };
        match &e.kind {
            ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw,
            } => {
                let v = hdl_ast::value_rules::decimal_i64(raw)?;
                leaf(v, 32, true)
            }
            ExprKind::IntLit {
                kind: kind @ (IntLitKind::Sized | IntLitKind::UnsizedBased),
                raw,
            } => {
                let sized = matches!(kind, IntLitKind::Sized);
                let (pat, lw) = Self::based_lit_pattern(raw, sized)?;
                let tick = raw.find('\'')?;
                let ls = matches!(raw[tick + 1..].chars().next(), Some('s') | Some('S'));
                let v = hdl_ast::value_rules::wrap_i128(pat as i128, lw, ls)?;
                leaf(v, lw, ls)
            }
            ExprKind::Ident(p) if p.segments.len() == 1 => {
                let c = self
                    .const_locals
                    .get(&p.segments[0].name)
                    .filter(|c| c.w > 0)?;
                leaf(c.v, c.w, c.signed)
            }
            ExprKind::PkgScoped { pkg, name } => {
                let c = self
                    .pkg_const_scoped
                    .get(&format!("{}::{}", pkg.name, name.name))
                    .filter(|c| c.w > 0)?;
                leaf(c.v, c.w, c.signed)
            }
            ExprKind::Paren { inner } => self.cw_eval(inner, w, signed),
            ExprKind::Unary {
                op: UnOp::Minus,
                operand,
            } => {
                let x = self.cw_eval(operand, w, signed)?;
                Some((-x) & mask)
            }
            ExprKind::SysCall { name, args } if name.name == "$clog2" && args.len() == 1 => {
                let n = self.try_const_index_wc(&args[0], 0)?;
                if n.v < 0 {
                    return None;
                }
                let v = i64::from(hdl_ast::value_rules::clog2_u64(n.v as u64));
                leaf(v, 32, true)
            }
            ExprKind::Binary { op, lhs, rhs } => {
                let x = self.cw_eval(lhs, w, signed)?;
                let y = self.cw_eval(rhs, w, signed)?;
                Some(hdl_ast::value_rules::ctx5_binop(*op, x, y, w, signed)? & mask)
            }
            _ => None,
        }
    }

    /// A constant BOUND a parse-time layout is built from: the width-aware fold
    /// where it applies, else the plain i64 fold (`try_const_index`).
    pub(crate) fn const_bound(&self, e: &Expr) -> Option<i64> {
        self.try_const_index_w(e)
            .map(|c| c.v)
            .or_else(|| self.try_const_index(e))
    }

    pub(crate) fn const_lit(e: &Expr) -> Option<i64> {
        match &e.kind {
            ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw,
            } => hdl_ast::value_rules::decimal_i64(raw),
            ExprKind::Unary {
                op: UnOp::Minus,
                operand,
            } => Self::const_lit(operand)?.checked_neg(),
            ExprKind::Binary { op, lhs, rhs } => {
                let a = Self::const_lit(lhs)?;
                let b = Self::const_lit(rhs)?;
                // Checked arithmetic: an overflowing constant fold returns None
                // (→ caller treats the value as non-foldable / loud) rather than
                // panicking in debug or silently wrapping in release.
                hdl_ast::value_rules::binop_in(hdl_ast::value_rules::OpSet::Lit3, *op, a, b)
            }
            _ => None,
        }
    }

    /// Fold an ENUM LABEL expression to `i64`, including sized / unsized-based
    /// literals (`4'h3`, `8'shFF`, `'b1011`) that [`Self::const_lit`] cannot see.
    ///
    /// `base_signed` is the ENUM BASE TYPE's signedness, and it — not the literal's
    /// own `s` marker — decides how the literal's bit pattern is read. §6.19: a
    /// label is a value OF the base type, so `enum integer { A = 32'hDEADBEEF }` is
    /// −559038737 (iverilog agrees) even though the literal is unsigned, and
    /// `enum bit [7:0] { A = 8'shFF }` is 255 even though the literal is signed.
    /// Folding the marker's sign instead rejected the first as out of range and gave
    /// the second a `.name()` table keyed on −1 while the constant was 255 — the
    /// label's NAME came back empty.
    ///
    /// Deliberately a SEPARATE entry point rather than a widening of `const_lit`:
    /// that one also decides packed-struct member layout and typedef ranges at parse
    /// time, so folding more there would move layout decisions across the front end.
    ///
    /// This is a SECOND place that turns literal text into a value — the first is
    /// `elaborate`'s `parse_int_literal`, unreachable from here (it lives in a crate
    /// that depends on this one). Two predicates for one value is a hazard: a
    /// disagreement shows up as a `.name()` table pointing at a different label than
    /// the constant, so only the NAME is wrong, silently. The accept-set is therefore
    /// narrowed to inputs where the two provably agree — see
    /// [`Self::based_lit_pattern`] — and `enum_sized_label.rs` pins every accepted
    /// form by printing `x.name()` and `x` together.
    pub(crate) fn const_lit_enum(e: &Expr, base_signed: bool) -> Option<i64> {
        match &e.kind {
            ExprKind::IntLit {
                kind: kind @ (IntLitKind::Sized | IntLitKind::UnsizedBased),
                raw,
            } => {
                let (pat, w) = Self::based_lit_pattern(raw, matches!(kind, IntLitKind::Sized))?;
                Self::read_pattern(pat, w, base_signed)
            }
            // `-4'sd1` negates WITHIN the literal's width (→ the 4-bit pattern
            // `1111`), then the base type reads it: 15 in an unsigned base.
            ExprKind::Unary {
                op: UnOp::Minus,
                operand,
            } => match &operand.kind {
                ExprKind::IntLit {
                    kind: kind @ (IntLitKind::Sized | IntLitKind::UnsizedBased),
                    raw,
                } => {
                    let (pat, w) = Self::based_lit_pattern(raw, matches!(kind, IntLitKind::Sized))?;
                    let mask = if w == 64 { !0u64 } else { (1u64 << w) - 1 };
                    Self::read_pattern((pat as i64).checked_neg()? as u64 & mask, w, base_signed)
                }
                _ => Self::const_lit(e),
            },
            _ => Self::const_lit(e),
        }
    }

    /// Read a `w`-bit pattern as a value of a signed / unsigned type.
    fn read_pattern(pat: u64, w: u32, signed: bool) -> Option<i64> {
        if w >= 64 {
            // Every 64-bit pattern is an i64 bit-for-bit. An UNSIGNED 64-bit value
            // above i64::MAX therefore lands here as a negative i64 — the label
            // table's own representation limit, tracked in ROADMAP §3.
            return Some(pat as i64);
        }
        if signed && (pat >> (w - 1)) & 1 == 1 {
            return Some((pat | (!0u64 << w)) as i64);
        }
        i64::try_from(pat).ok()
    }

    /// The `w`-bit unsigned PATTERN of a based literal `[W]'[s]B digits`, with its
    /// width. No sign extension happens here: the caller's type decides that.
    ///
    /// Declines (None) every input whose value this cannot reproduce EXACTLY the way
    /// `elaborate::parse_int_literal` does, which leaves the caller where it was:
    ///   * x/z/? digits, a fill literal (`'0`/`'1`/`'x`/`'z`) — no integer value;
    ///   * width 0 or above 64, or digits overflowing u64;
    ///   * digits that do NOT already fit the declared width. Reproducing truncation
    ///     is exactly where the two implementations part company (an UNSIZED based
    ///     literal grows past 32 bits there and would be masked to 32 here), so it
    ///     is refused rather than guessed;
    ///   * an UNSIZED literal carrying an `s` marker. Its width is 32 here but
    ///     `natural.max(32)` there, and the two only differ in whether bit 31 is a
    ///     sign bit — `'sd2147483648` would be −2147483648 here and +2147483648
    ///     there. Unsized WITHOUT `s` is safe: no sign bit is consulted, and the
    ///     fits-the-width rule above already forces the value below 2^32.
    pub(crate) fn based_lit_pattern(raw: &str, sized: bool) -> Option<(u64, u32)> {
        let tick = raw.find('\'')?;
        let width: u32 = if sized {
            let w = hdl_ast::value_rules::decimal_i64(&raw[..tick])
                .and_then(|v| u32::try_from(v).ok())?;
            if w == 0 || w > 64 {
                return None;
            }
            w
        } else {
            if !raw[..tick].is_empty() {
                return None;
            }
            32
        };
        let mut rest = raw[tick + 1..].chars().peekable();
        let signed_marker = matches!(rest.peek(), Some('s') | Some('S'));
        if signed_marker {
            rest.next();
            if !sized {
                return None; // unsized + `s`: width rules differ (see above)
            }
        }
        let base: u32 = match rest.next()? {
            'b' | 'B' => 2,
            'o' | 'O' => 8,
            'd' | 'D' => 10,
            'h' | 'H' => 16,
            _ => return None, // single-char fill: context-sized, no fixed value
        };
        let mut acc: u64 = 0;
        let mut any = false;
        for c in rest {
            if c == '_' {
                continue;
            }
            let d = c.to_digit(base)?; // x/z/? decline here
            acc = acc.checked_mul(base as u64)?.checked_add(d as u64)?;
            any = true;
        }
        if !any {
            return None;
        }
        let masked = if width == 64 {
            acc
        } else {
            acc & ((1u64 << width) - 1)
        };
        if masked != acc {
            return None; // truncation is declined, not reproduced
        }
        Some((masked, width))
    }

    /// A non-negative literal or `None`.
    pub(crate) fn lit_u32(e: &Expr) -> Option<u32> {
        hdl_ast::range_rules::lit_u32(Self::const_lit, e)
    }

    /// An unsized decimal or unsized based literal below 2^31 with no `x`, `z` or `?`
    /// digit; a fill (`'1`) is not one.
    pub(crate) fn small_unsized_lit(kind: IntLitKind, raw: &str) -> bool {
        let (radix, body) = match kind {
            IntLitKind::Decimal => (10, raw),
            IntLitKind::UnsizedBased => {
                let Some(rest) = raw.trim().strip_prefix('\'') else {
                    return false;
                };
                let rest = rest.strip_prefix(['s', 'S']).unwrap_or(rest);
                let mut ch = rest.chars();
                let radix = match ch.next() {
                    Some('b' | 'B') => 2,
                    Some('o' | 'O') => 8,
                    Some('d' | 'D') => 10,
                    Some('h' | 'H') => 16,
                    _ => return false,
                };
                (radix, ch.as_str())
            }
            IntLitKind::Sized => return false,
        };
        let digits: String = body
            .chars()
            .filter(|c| *c != '_' && !c.is_whitespace())
            .collect();
        u64::from_str_radix(&digits, radix).is_ok_and(|v| v <= i32::MAX as u64)
    }

    /// A decimal literal or its negation (parentheses peeled).
    pub(crate) fn plain_int(e: &Expr) -> Option<i64> {
        match &e.kind {
            ExprKind::Paren { inner } => Self::plain_int(inner),
            ExprKind::Unary {
                op: UnOp::Minus,
                operand,
            } => Self::plain_dec(operand)?.checked_neg(),
            _ => Self::plain_dec(e),
        }
    }

    /// A bare decimal literal (parentheses peeled): the one spelling whose value is the
    /// same self-determined and inside any wider offset arithmetic.
    pub(crate) fn plain_dec(e: &Expr) -> Option<i64> {
        match &e.kind {
            ExprKind::Paren { inner } => Self::plain_dec(inner),
            ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                ..
            } => Self::const_lit(e),
            _ => None,
        }
    }

    /// An index the literal arm takes: a decimal in `[0, count)`.
    pub(crate) fn literal_element(e: &Expr, count: u32) -> Option<u32> {
        decimal_literal(e)
            .filter(|v| (0..i64::from(count)).contains(v))
            .map(|v| v as u32)
    }

    /// §5.2 row 41: `e` is a decimal literal written in the source, its text its lexeme
    /// (`source_literal`), and its value; no sign, no operator, no fold.
    pub(crate) fn written_decimal(&self, e: &Expr) -> Option<i64> {
        match &e.kind {
            ExprKind::IntLit {
                kind: IntLitKind::Decimal,
                raw,
            } if self.src.get(e.span.lo as usize..e.span.hi as usize) == Some(raw.as_str()) => {
                Self::const_lit(e)
            }
            _ => None,
        }
    }

    /// §3 ⑤ ⓓ: could the parameter's declared type hold `v` unchanged? Declines
    /// only on a PROVABLE mismatch — a width this parser can fold that does not
    /// hold `v` under the declared signedness (`localparam byte B = 200` is −56 in
    /// elaborate, so folding 200 here would be a second evaluator disagreeing with
    /// the first). An untyped parameter takes the value's own type (§6.20.2) — a
    /// 32-bit signed integer for the decimal arithmetic `try_const_index` folds, so
    /// `v` must fit `i32`; `int`/`integer` are 32-bit (signed, or `unsigned`
    /// 0..2³²−1); `time` is 64-bit unsigned. A range whose bounds do not fold, and
    /// every other type (`real`, `string`, …), keep the pre-§4.5.414 behaviour —
    /// recorded as any `localparam` was (review B-2: declining those made two
    /// PRE-correct designs loud).
    pub(crate) fn const_param_fits(&self, p: &ParamDecl, v: i64) -> bool {
        let fits_width = |w: u32, signed: bool| -> bool {
            match (w, signed) {
                (0, _) | (65.., _) => false,
                (64, true) => true,
                (64, false) => v >= 0,
                (_, true) => {
                    let half = 1i64 << (w - 1);
                    v >= -half && v < half
                }
                (_, false) => v >= 0 && v < (1i64 << w),
            }
        };
        match (&p.ty, &p.dtype.range) {
            (ParamType::Implicit, None) => i32::try_from(v).is_ok(),
            (ParamType::Integer, None) => {
                if p.dtype.signed {
                    i32::try_from(v).is_ok()
                } else {
                    u32::try_from(v).is_ok()
                }
            }
            (ParamType::Time, None) => v >= 0,
            (ParamType::Implicit | ParamType::Integer, Some(r)) => {
                match (self.const_bound(&r.msb), self.const_bound(&r.lsb)) {
                    (Some(a), Some(b)) => {
                        let w = hdl_ast::range_rules::Lane::ParamFits.width_u32(a, b);
                        fits_width(w, p.dtype.signed)
                    }
                    _ => true,
                }
            }
            _ => true,
        }
    }

    /// §4.5.418: the value of a parameter whose initializer is ONE based literal
    /// (`parameter int unsigned W = 32'd12`, `'hFF`, `16'sh8000`). `try_const_index`
    /// deliberately does not fold based literals: inside an expression a literal
    /// narrower than 32 bits sets the expression's width, and the parse-time fold
    /// carries no width (ROADMAP §2: `[C+D:0]` over two 4-bit constants folds 17 where
    /// both oracles say 1). Standing alone, the literal's exact value is the value the
    /// declared type receives (`const_param_fits` then decides, as for a decimal).
    /// The parameter's WIDTH must be 32 bits or more — the literal's own width for
    /// an untyped parameter, the declared range or atom width otherwise — because
    /// the table folds a consumer's `[C+D:0]` at i64 and a narrower parameter wraps
    /// there (measured: `logic [3:0] C = 4'hf, D = 4'h1` → 17 where the oracles say
    /// 1; the decimal spelling is the same pre-existing silent, ROADMAP §2, and this
    /// spelling must not inherit it). A truncated (`4'd20`), x/z, or over-64-bit
    /// literal declines in `based_lit_pattern`; an unsigned 64-bit pattern above
    /// `i64::MAX` declines here (no i64 holds it).
    pub(crate) fn based_lit_param_value(&self, p: &ParamDecl) -> Option<(i64, u32, bool)> {
        let ExprKind::IntLit {
            kind: kind @ (IntLitKind::Sized | IntLitKind::UnsizedBased),
            raw,
        } = &p.value.kind
        else {
            return None;
        };
        let sized = matches!(kind, IntLitKind::Sized);
        let (pat, w) = Self::based_lit_pattern(raw, sized)?;
        let tick = raw.find('\'')?;
        let signed = matches!(raw[tick + 1..].chars().next(), Some('s') | Some('S'));
        let declared_w: u64 = match (&p.ty, &p.dtype.range) {
            (ParamType::Implicit, None) => u64::from(w),
            (ParamType::Integer, None) => 32,
            (ParamType::Time, None) => 64,
            (ParamType::Implicit | ParamType::Integer, Some(r)) => {
                let a = self.const_bound(&r.msb)?;
                let b = self.const_bound(&r.lsb)?;
                hdl_ast::range_rules::Lane::Wide64.width_u64(a, b)
            }
            _ => return None,
        };
        // (§4.5.418 declined a declaration narrower than 32 bits here because the
        // table's readers folded at i64; §2 🆕 L (aa) made them width-aware, so the
        // spelling is recorded like its decimal twin — `const_param_fits` still
        // refuses a literal the declared type does not hold.)
        let _ = declared_w;
        let v = hdl_ast::value_rules::wrap_i128(pat as i128, w, signed)?;
        Some((v, w, signed))
    }

    /// The WIDTH and sign the recorded constant's declaration gives it (IEEE §6.20 /
    /// §11.6): an untyped parameter takes its initializer's — 32-bit signed for a
    /// decimal, the literal's own for a lone based literal (`based`); `int` /
    /// `integer` are 32, `time` 64 unsigned, a declared range its folded width.
    /// `0` = unknown (the range did not fold): the width-aware twin declines there.
    pub(crate) fn param_const_shape(
        &self,
        p: &ParamDecl,
        based: Option<(i64, u32, bool)>,
    ) -> (u32, bool) {
        match (&p.ty, &p.dtype.range) {
            (ParamType::Implicit, None) => match based {
                Some((_, w, s)) => (w, s),
                None => (32, true),
            },
            (ParamType::Integer, None) => (32, p.dtype.signed),
            (ParamType::Time, None) => (64, false),
            (ParamType::Implicit | ParamType::Integer, Some(r)) => {
                match (self.const_bound(&r.msb), self.const_bound(&r.lsb)) {
                    (Some(a), Some(b)) => (
                        hdl_ast::range_rules::Lane::ParamSigned.width_u32(a, b),
                        p.dtype.signed,
                    ),
                    _ => (0, p.dtype.signed),
                }
            }
            _ => (0, p.dtype.signed),
        }
    }
}

/// A decimal literal as written, and nothing that folds to one.
fn decimal_literal(e: &Expr) -> Option<i64> {
    match &e.kind {
        ExprKind::IntLit {
            kind: IntLitKind::Decimal,
            ..
        } => Parser::const_lit(e),
        _ => None,
    }
}

/// `decimal_literal`, within `int`.
pub(crate) fn index_literal(e: &Expr) -> bool {
    decimal_literal(e).is_some_and(|v| v <= i64::from(i32::MAX))
}
