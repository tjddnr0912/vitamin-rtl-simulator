//! split part of `eval` (mechanical move).

use super::*;

/// 64-bit window of `plane` at an arbitrary BIT offset (two-word funnel; bits
/// beyond the slice read as 0). §A word化 helper, 2026-06-11.
#[inline]
pub(crate) fn window64(plane: &[u64], bit: u32) -> u64 {
    let w = (bit / 64) as usize;
    let sh = bit % 64;
    let lo = plane.get(w).copied().unwrap_or(0);
    if sh == 0 {
        return lo;
    }
    let hi = plane.get(w + 1).copied().unwrap_or(0);
    (lo >> sh) | (hi << (64 - sh))
}

/// Copy `w` bits of BOTH planes from `src[src_off..]` into `dst[dst_off..]`,
/// word-parallel. The destination range must be ZERO on entry (every caller
/// builds into a fresh `Value::zeros`) — bits are OR-merged in.
#[inline]
pub(crate) fn copy_bits(dst: &mut Value, dst_off: u32, src: &Value, src_off: u32, w: u32) {
    let mut i = 0u32;
    while i < w {
        let dbit = dst_off + i;
        let dw = (dbit / 64) as usize;
        let dsh = dbit % 64;
        let n = (64 - dsh).min(w - i);
        let m = if n == 64 { u64::MAX } else { (1u64 << n) - 1 };
        let sv = window64(&src.val, src_off + i) & m;
        let su = window64(&src.unk, src_off + i) & m;
        dst.val[dw] |= sv << dsh;
        dst.unk[dw] |= su << dsh;
        i += n;
    }
}

/// v6: the packed bits of a (fully-defined) value as a byte STRING — bytes
/// MSB-first with leading 0x00 bytes stripped (packed-ASCII surface, §6.16
/// conversion family). Shared by the string-assoc key eval and the iteration
/// methods' current-key read.
pub(crate) fn value_str_bytes(v: &Value) -> Vec<u8> {
    let nbytes = v.width.div_ceil(8);
    let mut out = Vec::with_capacity(nbytes as usize);
    let mut leading = true;
    for bi in (0..nbytes).rev() {
        let mut b: u8 = 0;
        for bit in 0..8u32 {
            let idx = bi * 8 + bit;
            if idx < v.width {
                let (val, _) = v.get_vu(idx);
                b |= (val as u8) << bit;
            }
        }
        if b == 0 && leading {
            continue; // strip leading nulls (width padding)
        }
        leading = false;
        out.push(b);
    }
    out
}

/// ⓑ-breadth (v18): parse the leading integer prefix of `bytes` in `radix`
/// (IEEE §6.16.9-12 `atoi`/`atohex`/`atooct`/`atobin`).
///
/// IEEE 1800 §6.16.9 states the rule exactly: "The conversion scans all leading
/// digits and underscore characters (`_`) and stops as soon as it encounters any
/// other character or the end of the string. It returns zero if no digits were
/// encountered." That is deliberately NOT `strtol`: there is no whitespace skipping
/// and no sign, and underscores are SCANNED (skipped in the value) rather than
/// terminating the scan.
///
/// R17: this was written as a `strtol`-style parser, with a comment asserting that
/// iverilog's stricter reading was "its bug". Measured against iverilog 13 and read
/// against the LRM, it was the other way round on three shapes — `" 3"` → 3 not 0,
/// `"-7"` → -7 not 0, `"1_0"` → 1 not 10 — all silently, all in the exact API the
/// round-17 reporter's `.rsp` header reader calls (`line.substr(a,b).atoi()`), where
/// a substring that happens to include the leading space of `"[L = 32]"` decided
/// whether a header field parsed at all. Both lenses agree, so the LRM wins over the
/// old comment.
///
/// Wrapping accumulation keeps the C-like overflow behavior the callers then truncate
/// to 32 bits (the LRM explicitly declines to define overflow here).
pub(crate) fn parse_radix_prefix(bytes: &[u8], radix: u32) -> i64 {
    let mut acc: i64 = 0;
    let mut any = false;
    for &b in bytes {
        // An underscore is a separator INSIDE the run of digits; it neither
        // contributes a value nor ends the scan. A leading underscore before any
        // digit is likewise scanned (the LRM says "leading digits and underscore
        // characters"), and still yields 0 when no digit ever follows.
        if b == b'_' {
            continue;
        }
        match (b as char).to_digit(radix) {
            Some(d) => {
                acc = acc.wrapping_mul(radix as i64).wrapping_add(d as i64);
                any = true;
            }
            None => break,
        }
    }
    if any {
        acc
    } else {
        0
    }
}

/// ⓑ-breadth (v18): parse the leading real prefix of `bytes` (IEEE §6.16.13
/// `atoreal`). Trims leading whitespace and parses the longest leading substring
/// that is a valid `f64`; non-numeric / empty yields 0.0.
pub(crate) fn parse_real_prefix(bytes: &[u8]) -> f64 {
    let s = String::from_utf8_lossy(bytes);
    let t = s.trim_start();
    // Longest valid f64 prefix (Rust's parser is whole-string, so shrink to fit).
    let end = t.len();
    for n in (1..=end).rev() {
        if t.is_char_boundary(n) {
            if let Ok(v) = t[..n].parse::<f64>() {
                return v;
            }
        }
    }
    0.0
}

/// Read-only net access the evaluator needs. The engine state implements it.
pub trait NetReader {
    /// Current 4-state value of net `net`, optional array word index.
    fn read_net(&self, net: u32, word: Option<u32>) -> Value;
    /// v5 (C): element count of the dynamic-storage object behind HANDLE net
    /// `net`. `Some(0)` for a declared-but-never-`new`ed handle (IEEE: empty),
    /// `None` when the net is not a dyn handle (the caller X-poisons). Default
    /// `None` keeps non-engine readers (native-eval test fakes) unchanged.
    fn dyn_size(&self, _net: u32) -> Option<u64> {
        None
    }
    /// ⓑ-breadth (v15): element-value snapshot of a dyn handle in deterministic
    /// order, for the array reduction/ordering/locator methods. `None` for a
    /// non-handle / string handle (the caller X-poisons); `Some(vec![])` for an
    /// empty array. Default `None` keeps non-engine readers (native-eval test
    /// fakes) unchanged.
    fn dyn_values(&self, _net: u32) -> Option<Vec<Value>> {
        None
    }
    /// ⓑ-breadth (v17): read the current `with`-clause iterator. `index=false`
    /// → the element value; `index=true` → the 0-based position (32-bit signed).
    /// Outside a fold the scratch is empty → X (defensive). Default X keeps
    /// non-engine readers unchanged.
    fn array_item(&self, _index: bool) -> Value {
        Value::xs(32, true)
    }
    /// ⓑ-breadth (v17): install a new with-clause iterator value (element +
    /// 0-based index) and return the previous one, so a fold can save/restore the
    /// scratch around each element evaluation (nested with-clauses bind their own).
    fn swap_array_item(&self, _v: Option<(Value, u64)>) -> Option<(Value, u64)> {
        None
    }
    /// v5 ④: report a dyn-storage degradation observed DURING eval (e.g. a
    /// queue pop in an unsupported placement). The engine latches it through
    /// the W4020 warn-once funnel; the no-op default keeps non-engine readers
    /// (native-eval test fakes) unchanged.
    fn dyn_warn(&self, _net: u32, _msg: &str) {}
    /// B1 frame-call: evaluate user function `func` with already-evaluated
    /// `args` (caller-context Values), returning the return-var Value. `None`
    /// (the default) ⇒ no frame-call support (native-eval test fakes / a Call
    /// with no sidecar entry) ⇒ the eval arm X-poisons. Only the engine
    /// (`SimState`) overrides this with the real frame evaluator.
    fn eval_call(&self, _func: u32, _args: &[Value]) -> Option<Value> {
        None
    }
    /// N7 virtual dispatch: given a method-call site (`call_eid`), its static
    /// target `static_fid`, and the already-evaluated `args` (args[0] = the
    /// receiver handle's object-id), return the FuncId to actually run. For a
    /// non-virtual site (no sidecar / fakes) this is `static_fid` (the default);
    /// the engine overrides it to index the receiver's runtime-class vtable.
    fn resolve_virtual_call(&self, _call_eid: u32, static_fid: u32, _args: &[Value]) -> u32 {
        static_fid
    }
    /// B1 frame-call: the i-th formal's (width, signed) so the eval arm can size
    /// each actual to the FORMAL type (IEEE 1800 §13.4.3) BEFORE the call. `None`
    /// (default / no sidecar) ⇒ fall back to the actual's self-width.
    fn formal_width(&self, _func: u32, _i: usize) -> Option<(u32, bool)> {
        None
    }
    /// B1 frame-call: is the i-th formal a `string` type? A string LITERAL actual
    /// evaluates to a packed const; a `string` formal must instead receive a
    /// heap-string value (the width-N slot would truncate the packed bits on
    /// read-back). Default `false` ⇒ non-engine readers never see string formals.
    fn formal_is_string(&self, _func: u32, _i: usize) -> bool {
        false
    }
    /// v5 ⑤: is `net` an ASSOC handle? Gates the i64-key read path in the
    /// Signal arm (assoc keys cannot ride the u32 word funnel). Default false
    /// — non-engine readers never see assoc nets.
    fn is_assoc(&self, _net: u32) -> bool {
        false
    }
    /// v5 ⑤: assoc-element read. `None` key = X/Z (invalid index). Only
    /// called where `is_assoc` returned true, so the default is unreachable
    /// by construction — it X-poisons defensively all the same.
    fn assoc_read(&self, _net: u32, _key: Option<i64>) -> Value {
        Value::xs(1, false)
    }
    /// v5 ⑤: `a.exists(k)` — `Some(true/false)` on an assoc handle (X/Z key
    /// matches nothing), `None` otherwise (the eval arm X-poisons).
    fn assoc_exists(&self, _net: u32, _key: Option<i64>) -> Option<bool> {
        None
    }
    /// v6: is `net` a STRING-keyed assoc handle? Gates the byte-key read path
    /// (checked BEFORE `is_assoc` in the Signal arm).
    fn is_assoc_str(&self, _net: u32) -> bool {
        false
    }
    /// v7 P2-C: the raw bytes of a STRING handle (`None` = not a string net).
    fn str_bytes(&self, _net: u32) -> Option<Vec<u8>> {
        None
    }
    /// v6: string-keyed element read — the byte twin of `assoc_read`.
    fn assoc_str_read(&self, _net: u32, _key: &Option<Vec<u8>>) -> Value {
        Value::xs(1, false)
    }
    /// v6: string-keyed `exists` — the byte twin of `assoc_exists`.
    fn assoc_str_exists(&self, _net: u32, _key: &Option<Vec<u8>>) -> Option<bool> {
        None
    }
    /// Round-9 FIO: the EOF flag of file descriptor `fd` as a 32-bit value
    /// (pre-opened → 0, bad/closed → −1, else 0/1), for a PURE expression-context
    /// `$feof(fd)`. Default X keeps non-engine readers (native-eval fakes) honest;
    /// `SimState` overrides with the live file state.
    fn fd_eof(&self, _fd: u32) -> Value {
        Value::xs(32, true)
    }
}

impl<N: NetReader> EvalCtx<'_, N> {
    /// Self-determined eval: size the node to its own self-width. Unchanged
    /// public surface; used by control-flow truthiness and systask args.
    pub fn eval(&self, eid: u32) -> Value {
        let sw = self.wt.get(eid);
        self.eval_ctx(eid, sw.width, sw.signed)
    }

    /// v7: decode an arg ExprId that is a string-literal Const (plusarg
    /// queries / formats). `None` for anything else.
    pub(crate) fn const_str_arg(&self, eid: u32) -> Option<String> {
        if let Expr::Const { val } = self.ir.exprs.get(eid as usize)? {
            let c = self.ir.consts.get(*val as usize)?;
            if c.repr == sim_ir::ConstRepr::StrUtf8 {
                return Some(crate::builtins::const_string(self.ir, *val));
            }
        }
        None
    }

    /// v7 P2-C: bytes of the STRING handle named by an arg ExprId (the
    /// `Signal{net, word:None}` elaborate emits for method receivers).
    pub(crate) fn handle_str_bytes(&self, arg: Option<&u32>) -> Option<Vec<u8>> {
        let &a = arg?;
        // A real string NET's heap bytes are authoritative (they may contain embedded
        // NULs that the packed `value_str_bytes` leading-strip would lose).
        if let Some(Expr::Signal { net, word: None }) = self.ir.exprs.get(a as usize) {
            if let Some(b) = self.nets.str_bytes(*net) {
                return Some(b);
            }
        }
        // Otherwise the operand is a string-VALUED expression whose packed bits ARE
        // the string — read them via `value_str_bytes` (MSB-first, leading width-pad
        // NULs stripped; same surface as the string assoc-key path, so it indexes
        // identically to a materialized net). Reaches here for a string `Const` (an
        // inline local bound to a literal) and a FRAME string formal (a 1-bit wire
        // whose frame slot holds the materialized string Value, `str_bytes` = None on
        // its non-`String` net kind). The elaborate side only routes genuine string
        // operands to the string primitives, so evaluating `a` as a string is sound.
        // Any X/Z (or a real) → `None` = no valid byte string.
        let v = self.eval(a);
        if v.is_real || v.has_xz() {
            None
        } else {
            Some(value_str_bytes(&v))
        }
    }

    /// v5 ⑤: evaluate an assoc KEY expression into the engine's signed-i64
    /// key domain — extend (sign- or zero-, per the expr's OWN signedness) to
    /// 64 bits, truncate anything wider (assignment-to-key-type semantics,
    /// §5.5; ⑥ elaborate casts the declared key type before the IR). Any X/Z
    /// (or a real) in the evaluated key → `None` = invalid index (§7.8.6).
    pub(crate) fn assoc_key(&self, eid: u32) -> Option<i64> {
        let sw = self.wt.get(eid);
        let v = self.eval_ctx(eid, sw.width.max(64), sw.signed);
        if v.is_real || v.has_xz() {
            return None;
        }
        Some(v.val.first().copied().unwrap_or(0) as i64)
    }

    /// v6: evaluate a STRING-assoc key expression into the byte-string key
    /// domain — self-determined eval, then the packed bits become bytes
    /// MSB-first with leading 0x00 bytes STRIPPED (packed-ASCII surface,
    /// §6.16 conversion family), so the same text at any padded width is the
    /// same key. X/Z anywhere (or a real) → `None` = invalid index.
    pub(crate) fn assoc_str_key(&self, eid: u32) -> Option<Vec<u8>> {
        let v = self.eval(eid);
        if v.is_real || v.has_xz() {
            return None;
        }
        Some(value_str_bytes(&v))
    }

    /// Evaluate `eid` in a context of at least `ctx_width` bits with context
    /// signedness `ctx_signed`. Returns a Value of width
    /// `max(self_width, ctx_width)`.
    ///
    /// CONTRACT:
    /// - context-determined nodes propagate `(max_width, AND-reduced signed)`
    ///   DOWN into their context-determined children (IEEE §5.4.1, §5.5.1);
    /// - self-determined nodes evaluate their children at the children's OWN
    ///   self-widths, produce the node's natural result, then resize the RESULT
    ///   to `ctx_width` using `ctx_signed` for the extension choice.
    pub fn eval_ctx(&self, eid: u32, ctx_width: u32, ctx_signed: bool) -> Value {
        let self_sw = self.wt.get(eid);
        // The evaluation width for THIS node and its context-determined children.
        let w = self_sw.width.max(ctx_width);
        // The global-unsigned rule (§5.5.1): once ANY operand in the
        // context-determined region is unsigned, the whole region is unsigned.
        // `eff_signed` = node self-signedness AND the context signedness.
        let eff_signed = self_sw.signed && ctx_signed;

        match &self.ir.exprs[eid as usize] {
            // ── leaves: read, then resize to `w` with eff_signed ───────────
            Expr::Const { val } => {
                let base = self.eval_const(*val);
                base.resize_keep_sign(w, eff_signed)
            }
            Expr::Signal { net, word } => {
                // v5 ⑤: assoc element — the key domain is SIGNED i64 (negative
                // and beyond-u32 keys are legal), so it must branch BEFORE the
                // u32 word funnel below. Scalar reads short-circuit on
                // `word.is_some()`; static arrays on the handle bitmap.
                if let Some(weid) = word {
                    // v6: the string-key twin branches first (disjoint kinds).
                    if self.nets.is_assoc_str(*net) {
                        let base = self.nets.assoc_str_read(*net, &self.assoc_str_key(*weid));
                        return base.resize_keep_sign(w, eff_signed);
                    }
                    if self.nets.is_assoc(*net) {
                        let base = self.nets.assoc_read(*net, self.assoc_key(*weid));
                        return base.resize_keep_sign(w, eff_signed);
                    }
                }
                // `word` is an ExprId (the array index expr), evaluated NOW so a
                // runtime `mem[k]` selects the right element. None ⇒ scalar/whole.
                // An X/Z index (`to_u64` → None) OR an index beyond u32 maps to
                // the `u32::MAX` out-of-range sentinel → `net_word_packed` returns
                // all-X — NOT a silent read of a wrapped element. Symmetric with
                // the write side (`resolve_lvalue_offsets`).
                let widx = word.map(|weid| {
                    self.eval(weid)
                        .to_u64()
                        .and_then(|v| u32::try_from(v).ok())
                        .unwrap_or(u32::MAX)
                });
                let base = self.nets.read_net(*net, widx);
                base.resize_keep_sign(w, eff_signed)
            }

            // ⓑ-breadth (v17): with-clause iterator — read the engine scratch.
            Expr::ArrayItem { index, .. } => {
                self.nets.array_item(*index).resize_keep_sign(w, eff_signed)
            }

            // ── unary ──────────────────────────────────────────────────────
            Expr::Unary { op, operand } => match op {
                // context-determined unary: propagate (w, eff_signed) into operand,
                // operate at w, result already w-wide.
                UnOp::Plus => self.eval_ctx(*operand, w, eff_signed),
                UnOp::Minus => {
                    let a = self.eval_ctx(*operand, w, eff_signed);
                    self.negate(&a) // width-preserving, stays `w`
                }
                UnOp::BitNot => {
                    // word-parallel 4-state complement; last partial word masked
                    // (`not_w` sets the high "0&0" region to 1).
                    let a = self.eval_ctx(*operand, w, eff_signed);
                    let mut r = Value::zeros(a.width, eff_signed);
                    for k in 0..nwords(a.width) {
                        let (v, u) = not_w(a.val[k], a.unk[k]);
                        let m = low_mask(a.width - 64 * k as u32);
                        r.val[k] = v & m;
                        r.unk[k] = u & m;
                    }
                    r
                }
                // reductions + lognot: SELF-DETERMINED operand, 1-bit result,
                // then zero-extend to `w` (= self_width(1).max(ctx_width), always
                // unsigned).
                UnOp::LogNot
                | UnOp::RedAnd
                | UnOp::RedNand
                | UnOp::RedOr
                | UnOp::RedNor
                | UnOp::RedXor
                | UnOp::RedXnor => {
                    let bit = self.eval_unary_self(*op, *operand); // 1-bit
                    bit.resize_keep_sign(w, false) // zero-extend
                }
            },

            // ── binary ─────────────────────────────────────────────────────
            Expr::Binary { op, lhs, rhs } => self.eval_binary_ctx(*op, *lhs, *rhs, w, eff_signed),

            // ── ternary: cond self-determined; branches context-determined ──
            Expr::Ternary {
                cond,
                then_e,
                else_e,
            } => match self.truthiness(&self.eval(*cond)) {
                Tri::True => self.eval_ctx(*then_e, w, eff_signed),
                Tri::False => self.eval_ctx(*else_e, w, eff_signed),
                Tri::Unknown => {
                    // both branches at (w, eff_signed); merge differing→X.
                    let t = self.eval_ctx(*then_e, w, eff_signed);
                    let e = self.eval_ctx(*else_e, w, eff_signed);
                    self.merge_x(&t, &e, w, eff_signed)
                }
            },

            // ── SELF-DETERMINED structural / select: eval natural, resize ──
            Expr::Concat { parts } => {
                let nat = self.eval_concat(parts); // sum of self-widths
                nat.resize_keep_sign(w, false) // concat unsigned
            }
            Expr::Replicate { count, value } => {
                let nat = self.eval_replicate(*count, *value);
                nat.resize_keep_sign(w, false) // replicate unsigned
            }
            Expr::Select {
                base,
                offset,
                width,
                kind,
            } => {
                let nat = self.eval_select(*base, *offset, *width, *kind); // unsigned
                nat.resize_keep_sign(w, false) // select unsigned
            }

            // ── system functions ───────────────────────────────────────────
            Expr::SysFunc { which, args } => self.eval_sysfunc_ctx(*which, args, w, eff_signed),

            // ── user function call (B1) ──────────────────────────────────────
            // Evaluate each actual at the FORMAL's width/sign (§13.4.3 — the
            // formal type is the assignment context), call the engine frame
            // evaluator, then resize the result to THIS call site's context.
            // `eval_call` returning `None` (empty sidecar / test fake) X-poisons
            // exactly like the pre-B1 stub, so func-free designs are unchanged.
            Expr::Call { func, args } => {
                let argv: Vec<Value> = args
                    .iter()
                    .enumerate()
                    .map(|(i, &a)| {
                        // A `string` formal lowers to a 1-bit Wire slot, so binding at
                        // the formal width would truncate the actual. Evaluate at the
                        // actual's NATURAL width (a string LITERAL is a packed const the
                        // 1-bit width would otherwise cut) and hand over a heap-string
                        // value — `resize_keep_sign` preserves `is_str`, so the slot
                        // never truncates it. A string VAR actual is already is_str, so
                        // this round-trips it unchanged.
                        if self.nets.formal_is_string(*func, i) {
                            let s = self.wt.get(a);
                            let v = self.eval_ctx(a, s.width.max(1), s.signed);
                            Value::from_str_bytes(&v.to_str_bytes())
                        } else {
                            let (fw, fs) = self.nets.formal_width(*func, i).unwrap_or_else(|| {
                                let s = self.wt.get(a);
                                (s.width, s.signed)
                            });
                            self.eval_ctx(a, fw, fs)
                        }
                    })
                    .collect();
                // N7: virtual dispatch redirects `func` to the receiver's runtime
                // class override; a non-virtual / non-class call keeps `*func`.
                let target = self.nets.resolve_virtual_call(eid, *func, &argv);
                match self.nets.eval_call(target, &argv) {
                    Some(r) => r.resize_keep_sign(w, eff_signed),
                    None => Value::x1().resize_keep_sign(w, false),
                }
            }
        }
    }

    /// Verilog truthiness of an expression: any definite-1 → true, all
    /// definite-0 → false, else (some x/z, no definite 1) → unknown.
    pub fn truthy(&self, eid: u32) -> bool {
        // X/Z is "false" for control flow (`if(x)` takes else). For logical
        // operators we use the tri-valued helper instead.
        matches!(self.truthiness(&self.eval(eid)), Tri::True)
    }

    pub(crate) fn eval_const(&self, cid: u32) -> Value {
        let c = &self.ir.consts[cid as usize];
        if matches!(c.repr, ConstRepr::Real) {
            // val[0] already holds f64::to_bits; reinterpret as real.
            return Value::from_f64(f64::from_bits(c.bits.val.first().copied().unwrap_or(0)));
        }
        let signed = matches!(c.repr, ConstRepr::Numeric) && c.signed;
        Value::from_packed(&c.bits, c.width, signed)
    }

    // ── Unary ──────────────────────────────────────────────────────────────

    /// 1-bit reduction/lognot result for a self-determined operand.
    pub(crate) fn eval_unary_self(&self, op: UnOp, operand: u32) -> Value {
        let a = self.eval(operand); // OWN self width
        match op {
            UnOp::LogNot => match self.truthiness(&a) {
                Tri::True => Value::zeros(1, false),
                Tri::False => Value::one1(),
                Tri::Unknown => Value::x1(),
            },
            UnOp::RedAnd => self.reduce_bit(&a, RedKind::And, false),
            UnOp::RedNand => self.reduce_bit(&a, RedKind::And, true),
            UnOp::RedOr => self.reduce_bit(&a, RedKind::Or, false),
            UnOp::RedNor => self.reduce_bit(&a, RedKind::Or, true),
            UnOp::RedXor => self.reduce_bit(&a, RedKind::Xor, false),
            UnOp::RedXnor => self.reduce_bit(&a, RedKind::Xor, true),
            _ => unreachable!("eval_unary_self only for reductions/lognot"),
        }
    }

    /// Word-parallel 4-state reduction → the single result bit `(v, u)`. Scans the
    /// val/unk plane words (the last masked to valid bits — a masked-out high bit
    /// must NOT read as a definite-0 and force AND→0), accumulating the three facts
    /// every reduction needs: any definite-0, any definite-1, any unknown, plus the
    /// definite-1 popcount for XOR parity. Semantics match the old per-bit fold:
    /// AND→0 if any 0 else x if any unknown else 1; OR dual; XOR→x if any unknown
    /// else parity.
    pub(crate) fn reduce_word(&self, a: &Value, kind: RedKind) -> (u64, u64) {
        if a.width == 0 {
            return (0, 0); // degenerate; matches the old zeros(1) seed
        }
        let mut any_unknown = false;
        let mut any_known1 = false;
        let mut any_known0 = false;
        let mut ones: u32 = 0;
        for k in 0..nwords(a.width) {
            let m = low_mask(a.width - 64 * k as u32);
            let av = a.val[k] & m;
            let au = a.unk[k] & m;
            let known1 = !au & av; // definite-1 bits (already within m)
            let known0 = !au & !av & m; // !av sets high bits → re-mask
            any_unknown |= au != 0;
            any_known0 |= known0 != 0;
            if known1 != 0 {
                any_known1 = true;
                ones += known1.count_ones();
            }
        }
        match kind {
            RedKind::And if any_known0 => (0, 0),
            RedKind::And if any_unknown => (0, 1),
            RedKind::And => (1, 0),
            RedKind::Or if any_known1 => (1, 0),
            RedKind::Or if any_unknown => (0, 1),
            RedKind::Or => (0, 0),
            RedKind::Xor if any_unknown => (0, 1),
            RedKind::Xor => ((ones & 1) as u64, 0),
        }
    }

    /// `reduce_word` wrapped into a 1-bit `Value`, optionally inverted (the N-forms
    /// RedNand/RedNor/RedXnor).
    pub(crate) fn reduce_bit(&self, a: &Value, kind: RedKind, neg: bool) -> Value {
        let (v, u) = self.reduce_word(a, kind);
        let (v, u) = if neg { not1((v, u)) } else { (v, u) };
        let mut r = Value::zeros(1, false);
        r.set_vu(0, v, u);
        r
    }

    pub(crate) fn negate(&self, a: &Value) -> Value {
        if a.is_real {
            // unwrap_or(0.0): on a real, to_f64 always returns Some, but we keep
            // the same unwrap policy everywhere to avoid a latent panic surface.
            return Value::from_f64(-a.to_f64().unwrap_or(0.0));
        }
        if a.has_xz() {
            return Value::xs(a.width, a.signed);
        }
        // Full-width two's complement (~x + 1 with word carry) — exact at any
        // width; the old single-word form left words 1+ zero for >64-bit
        // operands (P0-3).
        let mut out = Value::zeros(a.width, a.signed);
        let mut carry = 1u64;
        for k in 0..nwords(a.width).max(1) {
            let (s, c) = (!a.val.get(k).copied().unwrap_or(0)).overflowing_add(carry);
            out.val[k] = s;
            carry = c as u64;
        }
        out.mask_top();
        out
    }

    // ── Binary ─────────────────────────────────────────────────────────────

    /// Context-routed binary dispatch. `w` is the already-resolved eval width
    /// (= self_width.max(ctx_width)); the comparison/logical arms zero-extend
    /// their 1-bit result to `w` (= 1.max(ctx_width)).
    pub(crate) fn eval_binary_ctx(
        &self,
        op: BinOp,
        lhs: u32,
        rhs: u32,
        w: u32,
        eff_signed: bool,
    ) -> Value {
        use BinOp::*;
        match op {
            // ARITHMETIC — context-determined: BOTH operands sized to
            // (w, eff_signed), op at width w.
            Add | Sub | Mul | Div | Mod => {
                let l = self.eval_ctx(lhs, w, eff_signed);
                let r = self.eval_ctx(rhs, w, eff_signed);
                self.arith(op, &l, &r) // operates at max(l.w,r.w)=w
            }

            // POWER — base is context-determined; EXPONENT is SELF-DETERMINED.
            // `**` is signed iff the BASE is signed. The incoming `eff_signed`
            // (= base.self_signed AND ctx_signed) is already the base's effective
            // sign — the exponent never entered it. Evaluate the base in context;
            // the exponent is self-determined and its sign is restamped to the
            // base's so `arith`'s both-signed reduction follows the base.
            Pow => {
                let base = self.eval_ctx(lhs, w, eff_signed);
                // The exponent is SELF-determined: its value is read with its OWN
                // signedness. A narrow UNSIGNED exponent (`1'b1` = +1, `2'd2` = +2)
                // must NOT be reinterpreted as signed — widen it to `w` using its
                // own sign FIRST (so `1'b1` zero-extends to +1, a genuine `-1`
                // sign-extends to -1), THEN restamp to the base's sign so `arith`'s
                // both-signed reduction follows the base. (Restamping before the
                // widen turned `1'b1` into a 1-bit signed -1 ⇒ `2 ** 1'b1` = 0.)
                // Widen-ONLY: a bare `resize(w)` also TRUNCATED an exponent wider
                // than the result (`logic [3:0] r = a ** 18` read the exponent as
                // 18 mod 16 = 2 ⇒ 2**2 = 4, not 2**18 mod 16 = 0 — adversarial
                // review). The exponent's VALUE must survive intact; it is the
                // RESULT that wraps to the base's width, so widen to
                // max(w, own width) and truncate only after `arith`.
                let exp_raw = self.eval(rhs);
                let ew = exp_raw.width.max(w);
                let mut exp = exp_raw.resize(ew);
                exp.signed = base.signed;
                // IEEE Table 11-6: a 0 base with a NEGATIVE exponent is x (it is a
                // 0^(-k) division-by-zero). `2 ** -1` (|base| > 1) stays 0.
                if !base.has_xz()
                    && !exp.has_xz()
                    && base.to_u128() == Some(0)
                    && exp.to_i128_signed().map(|e| e < 0).unwrap_or(false)
                {
                    return Value::xs(w, base.signed);
                }
                // result width = base width = w (`arith` returns max(w, ew))
                self.arith(op, &base, &exp).resize(w)
            }

            // BITWISE — context-determined: BOTH operands sized to (w, eff_signed).
            BitAnd | BitOr | BitXor | BitXnor => {
                let l = self.eval_ctx(lhs, w, eff_signed);
                let r = self.eval_ctx(rhs, w, eff_signed);
                let f: WordBinOp = match op {
                    BitAnd => and_w,
                    BitOr => or_w,
                    BitXor => xor_w,
                    BitXnor => xnor_w,
                    _ => unreachable!("bitwise arm only handles BitAnd/Or/Xor/Xnor"),
                };
                self.bitwise(&l, &r, f)
            }

            // COMPARISONS / CASE-EQ — self-determined result (1-bit), but the two
            // operands are MUTUALLY context-determined: size each to
            // max(self_width(L), self_width(R)) with their pair-signedness. The
            // comparison does NOT inherit the enclosing ctx — this correctly stops
            // upward width/sign propagation.
            Lt | Le | Gt | Ge | Eq | Ne | CaseEq | CaseNe => {
                let cmp_w = self.wt.width(lhs).max(self.wt.width(rhs));
                let pair_signed = self.wt.signed(lhs) && self.wt.signed(rhs);
                let l = self.eval_ctx(lhs, cmp_w, pair_signed);
                let r = self.eval_ctx(rhs, cmp_w, pair_signed);
                let bit = match op {
                    CaseEq | CaseNe => self.case_eq(op, &l, &r),
                    Eq | Ne => self.log_eq(op, &l, &r),
                    _ => self.relational(op, &l, &r),
                };
                bit.resize_keep_sign(w, false) // zero-extend 1→w (= max(1,ctx))
            }

            // v7 casez/casex per-label match — same mutually-context-determined
            // operand sizing as the comparison class; known 0/1 result.
            CasezEq | CasexEq => {
                let cmp_w = self.wt.width(lhs).max(self.wt.width(rhs));
                let pair_signed = self.wt.signed(lhs) && self.wt.signed(rhs);
                let l = self.eval_ctx(lhs, cmp_w, pair_signed);
                let r = self.eval_ctx(rhs, cmp_w, pair_signed);
                self.casez_eq(op, &l, &r).resize_keep_sign(w, false)
            }

            // LOGICAL — self-determined operands, each reduced independently.
            LogAnd | LogOr => {
                let l = self.eval(lhs); // OWN self-width
                let r = self.eval(rhs);
                let bit = if matches!(op, LogAnd) {
                    self.log_and(&l, &r)
                } else {
                    self.log_or(&l, &r)
                };
                bit.resize_keep_sign(w, false) // = max(1, ctx)
            }

            // SHIFTS — LEFT operand is context-determined (result width = w);
            // RIGHT operand (amount) is SELF-DETERMINED (own width).
            Shl | AShl => {
                let l = self.eval_ctx(lhs, w, eff_signed); // widen LEFT FIRST
                let r = self.eval(rhs); // amount, own width
                let shifted = self.shift_left(&l, &r); // grows then we clamp
                shifted.resize_keep_sign(w, eff_signed) // back to ctx width
            }
            Shr => {
                let l = self.eval_ctx(lhs, w, eff_signed);
                let r = self.eval(rhs);
                self.shift_right(&l, &r, false) // logical, fill 0
            }
            // ARITHMETIC RIGHT SHIFT — the sign-fill is governed by the LEFT
            // operand's OWN self-signedness, NOT the enclosing context. An unsigned
            // enclosing context MUST NOT demote a genuinely-signed `s >>> n` to a
            // logical shift. Evaluate the LEFT operand with its OWN self-sign so its
            // MSB carries the true sign bit, and pass that same own-sign as the fill
            // flag; only AFTER shifting resize to the surrounding (w, eff_signed).
            AShr => {
                let lhs_signed = self.wt.signed(lhs); // OWN self-sign
                let l = self.eval_ctx(lhs, w, lhs_signed); // keep its OWN sign for fill MSB
                let r = self.eval(rhs);
                let shifted = self.shift_right(&l, &r, lhs_signed); // arith iff LEFT signed
                shifted.resize_keep_sign(w, eff_signed) // re-stamp to ctx sign
            }
        }
    }

    /// Element-wise 4-state bitwise op, computed 64 bits at a time over the
    /// val/unk plane words (was a per-bit `get_vu`/`set_vu` loop). The last partial
    /// word is masked to the valid bit count — `xnor_w` sets the high "0&0" region
    /// to 1, which would otherwise corrupt bits ≥ width. Verified bit-for-bit
    /// against the per-bit tables (`value::tests::word_vs_bit_parity`) and against
    /// the >64-bit X/Z cases (`bitwise_wide_xz_word_boundary`).
    pub(crate) fn bitwise(&self, l: &Value, r: &Value, f: WordBinOp) -> Value {
        let w = l.width.max(r.width);
        let both_signed = l.signed && r.signed;
        let le = l.clone().resize_keep_sign(w, both_signed);
        let re = r.clone().resize_keep_sign(w, both_signed);
        let mut out = Value::zeros(w, both_signed);
        let nw = nwords(w);
        for k in 0..nw {
            let (rv, ru) = f(le.val[k], le.unk[k], re.val[k], re.unk[k]);
            let m = low_mask(w - 64 * k as u32); // full word unless this is the last
            out.val[k] = rv & m;
            out.unk[k] = ru & m;
        }
        out
    }
}
