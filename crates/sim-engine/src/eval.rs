//! 4-state expression evaluator. Evaluates a `sim_ir::Expr` (by ExprId into
//! `SimIr.exprs`) to a [`Value`], reading nets via [`NetReader`] and consts from
//! `SimIr.consts`. IEEE-1364 4-state semantics; Z is normalized to X in every
//! operator except `===`/`!==`.
//!
//! v1 simplifications (documented, IEEE-permitted):
//! - any X/Z in an arithmetic operand poisons the whole result to X;
//! - the integer arithmetic lane is 64-bit (wider vectors truncate the numeric
//!   result — bitwise/concat/select/shift remain full-width).

use sim_ir::{BinOp, ConstRepr, Expr, SelKind, SimIr, SysFuncId, UnOp};

use crate::value::{and_w, low_mask, not1, not_w, nwords, or_w, xnor_w, xor_w, Value};

/// WIDE-ARITH-CAP: width above which the super-linear arithmetic kernels
/// (`*` O(n²), restoring `/`·`%` O(bits·n), `**` square-multiply) poison to X
/// instead of running. A declaration-legal net is ≤ `MAX_NET_WIDTH` (2^20), but
/// a replication concat (`{16{a}}`) can push an *operand* far past it (16M bits →
/// 34 s mul / 163 s div). Mirrors `elaborate`'s `MAX_NET_WIDTH` (same value), so
/// any operand within the declarable width regime is always computed exactly.
/// `simulate` warns once (W-RUN-WIDE-ARITH) when such a node exists.
pub(crate) const WIDE_ARITH_CAP: u32 = 1 << 20;

/// A word-parallel 4-state primitive: `(av,au, bv,bu) -> (rv,ru)`, 64 bits/op.
type WordBinOp = fn(u64, u64, u64, u64) -> (u64, u64);

/// Which reduction `reduce_word` performs (the N-forms negate the result).
#[derive(Clone, Copy)]
enum RedKind {
    And,
    Or,
    Xor,
}

/// 64-bit window of `plane` at an arbitrary BIT offset (two-word funnel; bits
/// beyond the slice read as 0). §A word化 helper, 2026-06-11.
#[inline]
fn window64(plane: &[u64], bit: u32) -> u64 {
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
fn copy_bits(dst: &mut Value, dst_off: u32, src: &Value, src_off: u32, w: u32) {
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
/// (IEEE §6.16.9-12 conversion family). Skips leading ASCII whitespace; honors a
/// single leading `+`/`-` when `signed`; accumulates `radix`-digits until the
/// first non-digit; empty/garbage yields 0. Wrapping accumulation matches the
/// C `strtol`-style overflow behavior the callers then truncate to 32 bits.
pub(crate) fn parse_radix_prefix(bytes: &[u8], radix: u32, signed: bool) -> i64 {
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    let mut neg = false;
    if signed && i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        neg = bytes[i] == b'-';
        i += 1;
    }
    let mut acc: i64 = 0;
    while i < bytes.len() {
        match (bytes[i] as char).to_digit(radix) {
            Some(d) => {
                acc = acc.wrapping_mul(radix as i64).wrapping_add(d as i64);
                i += 1;
            }
            None => break,
        }
    }
    if neg {
        acc.wrapping_neg()
    } else {
        acc
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
}

/// Evaluation context: the IR (consts/exprs), the net table, current time, and
/// the self-width side table that drives context-determined sizing.
pub struct EvalCtx<'a, N: NetReader> {
    pub ir: &'a SimIr,
    pub nets: &'a N,
    pub now: u64,
    pub wt: &'a crate::width::WidthTable,
    /// Time multiplier `M` of the process whose expression is being evaluated
    /// (`$time = now / M`, `$realtime = now / M` real). 1 ⇒ the 1ns/1ns base.
    pub time_mult: u64,
    /// v7 RNG state (`Cell`s — eval stays `&self`; every evaluation of
    /// `$random`/`$urandom` is a fresh draw, see `SimState::rng`).
    pub rng: &'a crate::state::RngCells,
    /// v7 runtime plusargs (CLI order — the $test/$value$plusargs search set).
    pub plusargs: &'a [String],
}

impl<'a, N: NetReader> EvalCtx<'a, N> {
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
    fn handle_str_bytes(&self, arg: Option<&u32>) -> Option<Vec<u8>> {
        let net = arg.and_then(|&a| match self.ir.exprs.get(a as usize) {
            Some(Expr::Signal { net, word: None }) => Some(*net),
            _ => None,
        })?;
        self.nets.str_bytes(net)
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
                        let (fw, fs) = self.nets.formal_width(*func, i).unwrap_or_else(|| {
                            let s = self.wt.get(a);
                            (s.width, s.signed)
                        });
                        self.eval_ctx(a, fw, fs)
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

    fn eval_const(&self, cid: u32) -> Value {
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
    fn eval_unary_self(&self, op: UnOp, operand: u32) -> Value {
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
    fn reduce_word(&self, a: &Value, kind: RedKind) -> (u64, u64) {
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
    fn reduce_bit(&self, a: &Value, kind: RedKind, neg: bool) -> Value {
        let (v, u) = self.reduce_word(a, kind);
        let (v, u) = if neg { not1((v, u)) } else { (v, u) };
        let mut r = Value::zeros(1, false);
        r.set_vu(0, v, u);
        r
    }

    fn negate(&self, a: &Value) -> Value {
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
    fn eval_binary_ctx(&self, op: BinOp, lhs: u32, rhs: u32, w: u32, eff_signed: bool) -> Value {
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
                let mut exp = self.eval(rhs).resize(w);
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
                self.arith(op, &base, &exp) // result width = base width = w
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
    fn bitwise(&self, l: &Value, r: &Value, f: WordBinOp) -> Value {
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

    fn arith(&self, op: BinOp, l: &Value, r: &Value) -> Value {
        if l.is_real || r.is_real {
            // IEEE 1364 §4.3: if either operand is real, the other promotes to real.
            // An X/Z integer entering a mixed real op decays to 0.0 (documented MVP
            // policy), never panics, never X-propagates.
            let a = l.to_f64().unwrap_or(0.0);
            let b = r.to_f64().unwrap_or(0.0);
            let res = match op {
                BinOp::Add => a + b,
                BinOp::Sub => a - b,
                BinOp::Mul => a * b,
                BinOp::Div => a / b, // f64: x/0 → ±inf, 0/0 → NaN; NOT X
                // `%` and `**` on a real are permanent illegalities gated at
                // elaborate (§6.2). Defensive NaN poison instead of unreachable!()
                // so a gate regression can never crash the simulator.
                BinOp::Mod => f64::NAN,
                BinOp::Pow => f64::NAN,
                _ => f64::NAN,
            };
            return Value::from_f64(res);
        }
        let w = l.width.max(r.width).max(1);
        let both_signed = l.signed && r.signed;
        if l.has_xz() || r.has_xz() {
            return Value::xs(w, both_signed);
        }
        // Arithmetic lane: 128 bits. SIGNED stays a 64-bit lane — sign
        // reconstruction (`to_i128_signed`) gates on width≤64, and a >64-bit signed
        // value would read mis-signed; poison to X (an honest "unsupported" beats a
        // silently wrong number). UNSIGNED now spans the full 128-bit u128 lane (so
        // a `[127:0]` add/mul carries past bit 63 correctly); only width>128 — beyond
        // the lane — poisons, mirroring the signed guard rather than truncating.
        // Phase-1.x ⑥: beyond the native lanes (signed >64 / unsigned >128)
        // arithmetic computes EXACTLY on the word grid (iverilog-differential)
        // — these used to X-poison as an honest "unsupported".
        if (both_signed && w > 64) || (!both_signed && w > 128) {
            return self.arith_wide(op, l, r, w, both_signed);
        }
        let res: u128 = if both_signed {
            let a = l
                .clone()
                .resize_keep_sign(w, true)
                .to_i128_signed()
                .unwrap();
            let b = r
                .clone()
                .resize_keep_sign(w, true)
                .to_i128_signed()
                .unwrap();
            match op {
                BinOp::Add => a.wrapping_add(b) as u128,
                BinOp::Sub => a.wrapping_sub(b) as u128,
                BinOp::Mul => a.wrapping_mul(b) as u128,
                BinOp::Div => {
                    if b == 0 {
                        return Value::xs(w, true);
                    }
                    a.wrapping_div(b) as u128
                }
                BinOp::Mod => {
                    if b == 0 {
                        return Value::xs(w, true);
                    }
                    a.wrapping_rem(b) as u128
                }
                BinOp::Pow => ipow_signed(a, b),
                _ => unreachable!(),
            }
        } else {
            let a = l.to_u128().unwrap();
            let b = r.to_u128().unwrap();
            match op {
                BinOp::Add => a.wrapping_add(b),
                BinOp::Sub => a.wrapping_sub(b),
                BinOp::Mul => a.wrapping_mul(b),
                BinOp::Div => {
                    if b == 0 {
                        return Value::xs(w, false);
                    }
                    a / b
                }
                BinOp::Mod => {
                    if b == 0 {
                        return Value::xs(w, false);
                    }
                    a % b
                }
                // Square-and-multiply mod 2^128 (then masked to `w`) — a^n WRAPS
                // mod 2^w like iverilog, instead of the old `checked_pow(..)
                // .unwrap_or(0)` that returned 0 on u128 overflow (`64'hF..F ** 3`
                // is all-ones, not 0). For w*n <= 128 the value never wraps, so
                // this is byte-identical to the old result there.
                BinOp::Pow => {
                    let mut acc: u128 = 1;
                    let mut base = a;
                    let mut e = b;
                    while e > 0 {
                        if e & 1 == 1 {
                            acc = acc.wrapping_mul(base);
                        }
                        e >>= 1;
                        if e > 0 {
                            base = base.wrapping_mul(base);
                        }
                    }
                    acc
                }
                _ => unreachable!(),
            }
        };
        // Store the low 128 bits across word 0 (and word 1 for w>64); `mask_top`
        // clears bits above `w`.
        let mut out = Value::zeros(w, both_signed);
        out.val[0] = res as u64;
        if nwords(w) > 1 {
            if out.val.len() < 2 {
                out.val.resize(2, 0);
            }
            out.val[1] = (res >> 64) as u64;
        }
        out.mask_top();
        out
    }

    /// Multi-word arithmetic (Phase-1.x ⑥) for widths beyond the native
    /// lanes. Operands are X-free (gated by the caller); both extend to the
    /// w-bit grid (sign-extending only when BOTH are signed, §4.5) and every
    /// op computes mod 2^w in two's complement — school multiplication,
    /// short (one-word divisor) or restoring long division, square-multiply
    /// power. Division signs per IEEE: quotient truncates toward zero, the
    /// remainder takes the DIVIDEND's sign.
    fn arith_wide(&self, op: BinOp, l: &Value, r: &Value, w: u32, both_signed: bool) -> Value {
        // WIDE-ARITH-CAP: the super-linear kernels would stall for tens of seconds
        // once a replication concat pushes an operand past the cap. Poison to X
        // above it (the div-by-zero degrade precedent); Add/Sub are O(n) and stay
        // exact at any width. The matching loud warning is emitted once in
        // `simulate` (W-RUN-WIDE-ARITH) so the degradation is never silent.
        if w > WIDE_ARITH_CAP && matches!(op, BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow) {
            return Value::xs(w, both_signed);
        }
        let n = nwords(w).max(1);
        let le = l.clone().resize_keep_sign(w, both_signed);
        let re = r.clone().resize_keep_sign(w, both_signed);
        let a: Vec<u64> = (0..n)
            .map(|k| le.val.get(k).copied().unwrap_or(0))
            .collect();
        let b: Vec<u64> = (0..n)
            .map(|k| re.val.get(k).copied().unwrap_or(0))
            .collect();
        let sa = both_signed && le.get_vu(w - 1).0 == 1;
        let sb = both_signed && re.get_vu(w - 1).0 == 1;
        let words = match op {
            BinOp::Add => mw_mask(mw_add(&a, &b), w),
            BinOp::Sub => mw_mask(mw_add(&a, &mw_neg(&b)), w),
            BinOp::Mul => mw_mask(mw_mul(&a, &b, n), w),
            BinOp::Div | BinOp::Mod => {
                if mw_is_zero(&b) {
                    return Value::xs(w, both_signed);
                }
                let ma = if sa {
                    mw_mask(mw_neg(&a), w)
                } else {
                    a.clone()
                };
                let mb = if sb {
                    mw_mask(mw_neg(&b), w)
                } else {
                    b.clone()
                };
                let (q, rem) = mw_divmod(&ma, &mb);
                if op == BinOp::Div {
                    let neg = sa != sb;
                    mw_mask(if neg { mw_neg(&q) } else { q }, w)
                } else {
                    mw_mask(if sa { mw_neg(&rem) } else { rem }, w)
                }
            }
            BinOp::Pow => {
                if sb {
                    // negative exponent (IEEE 1364 table): 1 → 1; -1 → ±1 by
                    // exponent parity; 0 → X; |base| > 1 → 0.
                    let one = mw_one(n);
                    let minus_one = mw_mask(mw_neg(&one), w);
                    if a == one {
                        one
                    } else if a == minus_one {
                        if b[0] & 1 == 0 {
                            one
                        } else {
                            minus_one
                        }
                    } else if mw_is_zero(&a) {
                        return Value::xs(w, both_signed);
                    } else {
                        vec![0; n]
                    }
                } else {
                    mw_pow(&a, &b, w)
                }
            }
            _ => unreachable!(),
        };
        let mut out = Value::zeros(w, both_signed);
        for (k, &wd) in words.iter().enumerate().take(n) {
            out.val[k] = wd;
        }
        out.mask_top();
        out
    }

    fn relational(&self, op: BinOp, l: &Value, r: &Value) -> Value {
        if l.is_real || r.is_real {
            let a = l.to_f64().unwrap_or(0.0);
            let b = r.to_f64().unwrap_or(0.0);
            let bit = match (op, a.partial_cmp(&b)) {
                // partial_cmp is None on NaN → all ordered comparisons false (IEEE).
                (_, None) => false,
                (BinOp::Lt, Some(o)) => o == std::cmp::Ordering::Less,
                (BinOp::Le, Some(o)) => o != std::cmp::Ordering::Greater,
                (BinOp::Gt, Some(o)) => o == std::cmp::Ordering::Greater,
                (BinOp::Ge, Some(o)) => o != std::cmp::Ordering::Less,
                _ => unreachable!("relational only handles Lt/Le/Gt/Ge"),
            };
            return Value::logic(bit);
        }
        if l.has_xz() || r.has_xz() {
            return Value::x1();
        }
        // Exact word-wise compare at ANY width (no 64/128-bit lane): extend both
        // operands to the common width (§4.5: sign-extend only when BOTH signed),
        // then compare. For equal-width same-sign two's-complement values the
        // plain lexicographic word order IS the numeric order; differing sign
        // bits decide directly. Fixes the silent low-word truncation (P0-1).
        use std::cmp::Ordering::*;
        let w = l.width.max(r.width).max(1);
        let both_signed = l.signed && r.signed;
        let le = l.clone().resize_keep_sign(w, both_signed);
        let re = r.clone().resize_keep_sign(w, both_signed);
        let cmp_words = |a: &Value, b: &Value| {
            let n = a.val.len().max(b.val.len());
            for k in (0..n).rev() {
                let av = a.val.get(k).copied().unwrap_or(0);
                let bv = b.val.get(k).copied().unwrap_or(0);
                match av.cmp(&bv) {
                    Equal => continue,
                    o => return o,
                }
            }
            Equal
        };
        let ord = if both_signed {
            match (le.get_vu(w - 1).0, re.get_vu(w - 1).0) {
                (1, 0) => Less,
                (0, 1) => Greater,
                _ => cmp_words(&le, &re),
            }
        } else {
            cmp_words(&le, &re)
        };
        let b = matches!(
            (op, ord),
            (BinOp::Lt, Less)
                | (BinOp::Le, Less)
                | (BinOp::Le, Equal)
                | (BinOp::Gt, Greater)
                | (BinOp::Ge, Greater)
                | (BinOp::Ge, Equal)
        );
        Value::logic(b)
    }

    /// `==` / `!=`: any compared bit x/z → X; else bit-equality.
    ///
    /// Width unification follows IEEE 1364-2001 §4.5: the comparison is signed
    /// ONLY when BOTH operands are signed; if either is unsigned both operands
    /// zero-extend. Using `resize` (which honors each operand's *own* sign) would
    /// sign-extend a lone signed operand in an unsigned context and report a false
    /// match (e.g. `4'sb1111 == 8'hFF` → wrong `1`). `resize_keep_sign` clears the
    /// sign when the context is unsigned, so we zero-extend correctly.
    fn log_eq(&self, op: BinOp, l: &Value, r: &Value) -> Value {
        if l.is_real || r.is_real {
            let a = l.to_f64().unwrap_or(0.0);
            let b = r.to_f64().unwrap_or(0.0);
            // VALUE comparison: +0.0 == -0.0 is true; NaN != NaN.
            let eq = a == b;
            return Value::logic(if op == BinOp::Eq { eq } else { !eq });
        }
        let w = l.width.max(r.width);
        let ctx_signed = l.signed && r.signed;
        let le = l.clone().resize_keep_sign(w, ctx_signed);
        let re = r.clone().resize_keep_sign(w, ctx_signed);
        // Word-parallel (was a per-bit `get_vu` loop): any x/z on either side
        // (`unk`) poisons the result to X; otherwise compare the val planes.
        // `resize_keep_sign` canonicalizes both operands (planes masked past
        // `width`), so a word-wise scan is bit-exact for the live width.
        let mut unk = 0u64;
        let mut diff = 0u64;
        for k in 0..nwords(w) {
            let lu = le.unk.get(k).copied().unwrap_or(0);
            let ru = re.unk.get(k).copied().unwrap_or(0);
            let lv = le.val.get(k).copied().unwrap_or(0);
            let rv = re.val.get(k).copied().unwrap_or(0);
            unk |= lu | ru;
            diff |= lv ^ rv;
        }
        if unk != 0 {
            return Value::x1();
        }
        let eq = diff == 0;
        Value::logic(if op == BinOp::Eq { eq } else { !eq })
    }

    /// `===` / `!==`: exact 4-state per-bit compare, never X. Width unification
    /// uses the same context-signedness rule as `==` (zero-extend unless BOTH
    /// signed) so a mixed-sign `===` matches IEEE numeric extension.
    fn case_eq(&self, op: BinOp, l: &Value, r: &Value) -> Value {
        if l.is_real || r.is_real {
            // MVP: === on real == VALUE equality. A real is 2-state, so === and ==
            // coincide; +0.0 === -0.0 is TRUE (value equal), NaN !== NaN.
            let a = l.to_f64().unwrap_or(0.0);
            let b = r.to_f64().unwrap_or(0.0);
            let eq = a == b;
            return Value::logic(if op == BinOp::CaseEq { eq } else { !eq });
        }
        let w = l.width.max(r.width);
        let ctx_signed = l.signed && r.signed;
        let le = l.clone().resize_keep_sign(w, ctx_signed);
        let re = r.clone().resize_keep_sign(w, ctx_signed);
        // Word-parallel exact 4-state compare (both planes), canonical after
        // `resize_keep_sign`; was a per-bit `get_vu` loop.
        let mut neq = 0u64;
        for k in 0..nwords(w) {
            let lv = le.val.get(k).copied().unwrap_or(0);
            let rv = re.val.get(k).copied().unwrap_or(0);
            let lu = le.unk.get(k).copied().unwrap_or(0);
            let ru = re.unk.get(k).copied().unwrap_or(0);
            neq |= (lv ^ rv) | (lu ^ ru);
        }
        let eq = neq == 0;
        Value::logic(if op == BinOp::CaseEq { eq } else { !eq })
    }

    /// v7 casez/casex per-label match (IEEE 1364 §9.5.1, live-pinned against
    /// iverilog). A bit is don't-care iff EITHER side is z (`CasezEq`) or
    /// x-or-z (`CasexEq`); every remaining position compares 4-state EXACT
    /// (val AND unk planes equal — so an explicit x in a casez label matches
    /// only an x). Word-parallel; the result is always known 0/1.
    /// Encoding reminder: x = (val 0, unk 1), z = (val 1, unk 1).
    fn casez_eq(&self, op: BinOp, l: &Value, r: &Value) -> Value {
        let n = nwords(l.width.max(r.width)).max(1);
        for k in 0..n {
            let lv = l.val.get(k).copied().unwrap_or(0);
            let lu = l.unk.get(k).copied().unwrap_or(0);
            let rv = r.val.get(k).copied().unwrap_or(0);
            let ru = r.unk.get(k).copied().unwrap_or(0);
            let dc = if op == BinOp::CasezEq {
                (lu & lv) | (ru & rv) // z on either side
            } else {
                lu | ru // x OR z on either side
            };
            // 4-state exact mismatch on a non-don't-care position. mask_top
            // keeps both planes zero past `width`, so no spurious top bits.
            if !dc & ((lv ^ rv) | (lu ^ ru)) != 0 {
                return Value::logic(false);
            }
        }
        Value::logic(true)
    }

    fn log_and(&self, l: &Value, r: &Value) -> Value {
        match (self.truthiness(l), self.truthiness(r)) {
            (Tri::False, _) | (_, Tri::False) => Value::zeros(1, false),
            (Tri::True, Tri::True) => Value::one1(),
            _ => Value::x1(),
        }
    }

    fn log_or(&self, l: &Value, r: &Value) -> Value {
        match (self.truthiness(l), self.truthiness(r)) {
            (Tri::True, _) | (_, Tri::True) => Value::one1(),
            (Tri::False, Tri::False) => Value::zeros(1, false),
            _ => Value::x1(),
        }
    }

    fn shift_left(&self, l: &Value, r: &Value) -> Value {
        if r.has_xz() {
            return Value::xs(l.width, l.signed);
        }
        // An amount that doesn't fit u64 is astronomically larger than any net
        // width — saturate so everything shifts out (was: silent low-word use).
        let amt = r.to_u64().unwrap_or(u64::MAX);
        // v1 has no context-determined width (elaborate defers expr sizing), so a
        // self-determined `<<` would truncate to `l.width` and drop bits that a
        // wider assignment context would keep (`4'b0001 << 5` → 0 instead of
        // `8'h20`). We GROW the result to `l.width + amt` so no bit is ever lost;
        // the enclosing `write_lvalue`/operator then truncates to the real LHS
        // width. This is lossless and matches any context at least that wide;
        // narrower contexts truncate identically either way. Cap the growth so a
        // pathological shift amount can't allocate unboundedly.
        let grow = (l.width as u64).saturating_add(amt).min(4096) as u32;
        let w = grow.max(l.width).max(1);
        l.shl_grow(amt, w) // word-parallel (vacated low bits = 0)
    }

    fn shift_right(&self, l: &Value, r: &Value, arith: bool) -> Value {
        if r.has_xz() {
            return Value::xs(l.width, l.signed);
        }
        // Over-u64 amount ⇒ saturate (shift everything out / full sign fill).
        let amt = r.to_u64().unwrap_or(u64::MAX);
        let w = l.width;
        let (fv, fu) = if arith && w > 0 {
            l.get_vu(w - 1)
        } else {
            (0, 0)
        };
        l.shr_fill(amt, w, fv, fu) // word-parallel (top fill = sign for arith, else 0)
    }

    // ── Ternary ────────────────────────────────────────────────────────────

    /// Merge two equal-width branches bit-by-bit: agreeing bits pass through,
    /// differing bits become X. Both `t`/`e` are already `w`-wide from
    /// `eval_ctx`, so no inner resize is needed (verbatim former eval_ternary
    /// unknown-branch body).
    fn merge_x(&self, t: &Value, e: &Value, w: u32, signed: bool) -> Value {
        // WORD-PARALLEL X-merge (§A word化, 2026-06-11 — was bit-serial):
        // a result bit keeps the operand bit where BOTH planes agree and
        // X-poisons where they differ. Bits beyond an operand's width read
        // as (0,0), exactly like the old `get_vu` path (mask_top invariant).
        let mut out = Value::zeros(w, signed);
        let n = crate::value::nwords(w).max(1);
        for k in 0..n {
            let tv = t.val.get(k).copied().unwrap_or(0);
            let tu = t.unk.get(k).copied().unwrap_or(0);
            let ev = e.val.get(k).copied().unwrap_or(0);
            let eu = e.unk.get(k).copied().unwrap_or(0);
            let eq = !((tv ^ ev) | (tu ^ eu));
            out.val[k] = tv & eq;
            out.unk[k] = (tu & eq) | !eq;
        }
        let m = crate::value::top_mask(w);
        out.val[n - 1] &= m;
        out.unk[n - 1] &= m;
        out
    }

    // ── Concat / Replicate ─────────────────────────────────────────────────

    fn eval_concat(&self, parts: &[u32]) -> Value {
        let vals: Vec<Value> = parts.iter().map(|&p| self.eval(p)).collect();
        let total: u32 = vals.iter().map(|v| v.width).sum();
        let mut out = Value::zeros(total.max(1), false);
        out.width = total;
        // parts[0] is MSB-most; fill from the top down — word-parallel copy
        // (§A word化, 2026-06-11; was a per-bit set_vu loop).
        let mut pos = total;
        for v in &vals {
            pos -= v.width;
            copy_bits(&mut out, pos, v, 0, v.width);
        }
        out.mask_top();
        out
    }

    fn eval_replicate(&self, count: u32, value: u32) -> Value {
        // `count` is an ExprId (frozen IR: Replicate.count is a const-expr edge),
        // NOT a literal — fold it to the repeat count, symmetric with the
        // self-width table (width.rs) and eval_select's width fold.
        let count = crate::width::const_u32_of_expr(self.ir, count).unwrap_or(0);
        let v = self.eval(value);
        let total = v.width.saturating_mul(count);
        let mut out = Value::zeros(total.max(1), false);
        out.width = total;
        // word-parallel per repetition (§A word化, 2026-06-11).
        for c in 0..count {
            copy_bits(&mut out, c * v.width, &v, 0, v.width);
        }
        out.mask_top();
        out
    }

    // ── Select ─────────────────────────────────────────────────────────────

    fn eval_select(&self, base: u32, offset: u32, width: u32, kind: SelKind) -> Value {
        // `width` is an ExprId (frozen IR: `Select.width` is a const-expr edge,
        // e.g. `Add(Sub(msb,lsb),1)`), NOT a literal bit count — fold it to its
        // value. `offset` stays an evaluated expr (it is the runtime index for
        // indexed `[base +: w]`/`[base -: w]` selects).
        let width = crate::width::const_u32_of_expr(self.ir, width).unwrap_or(1);
        let src = self.eval(base);
        let off_val = self.eval(offset);
        let off = match off_val.to_u64().and_then(|o| i64::try_from(o).ok()) {
            Some(o) => o,
            // X/Z offset or one beyond the i64 lane: the select is out of range.
            None => return Value::xs(width.max(1), false),
        };
        let (lsb, w) = match kind {
            SelKind::Bit => (off, 1u32),
            SelKind::PartConst | SelKind::PartIdxUp => (off, width),
            SelKind::PartIdxDown => (off - (width as i64) + 1, width),
        };
        let mut out = Value::zeros(w.max(1), false);
        out.width = w;
        // Fully in-range select: ONE word-parallel copy (§A word化,
        // 2026-06-11 — the dominant case). Any out-of-range overlap keeps
        // the per-bit path (mixed copied/X-filled bits).
        if lsb >= 0 && (lsb as u64) + (w as u64) <= src.width as u64 {
            copy_bits(&mut out, 0, &src, lsb as u32, w);
            out.mask_top();
            return out;
        }
        for i in 0..w as i64 {
            let src_idx = lsb + i;
            if src_idx >= 0 && (src_idx as u32) < src.width {
                let (v, u) = src.get_vu(src_idx as u32);
                out.set_vu(i as u32, v, u);
            } else {
                out.set_vu(i as u32, 0, 1); // out-of-range read → X
            }
        }
        out.mask_top();
        out
    }

    // ── SysFunc ────────────────────────────────────────────────────────────

    fn eval_sysfunc(&self, which: SysFuncId, args: &[u32]) -> Value {
        match which {
            // v5 (C)-③: `.size()` of a dyn handle. The arg is the handle's
            // Signal expr — resolve the NetId and ask the reader; a non-handle
            // (or a non-engine reader) X-poisons defensively, never panics.
            SysFuncId::DynSize => {
                let net = args
                    .first()
                    .and_then(|&a| match self.ir.exprs.get(a as usize) {
                        Some(Expr::Signal { net, word: None }) => Some(*net),
                        _ => None,
                    });
                match net.and_then(|n| self.nets.dyn_size(n)) {
                    Some(n) => {
                        let mut v = Value::zeros(32, true);
                        v.val[0] = n.min(i32::MAX as u64);
                        v
                    }
                    None => Value::xs(32, true),
                }
            }
            // v5 ④: queue pops are SIDE-EFFECTING — legal only as the DIRECT
            // rhs of a blocking assign, where the executor intercepts them
            // BEFORE eval (`StmtEffect::QPop`). Reaching THIS arm means an
            // unsupported placement (NBA rhs, nested expr, $monitor arg, …):
            // degrade LOUDLY to element-width X and do NOT pop — eval is the
            // pure READ phase (P7a) and must never mutate the heap.
            SysFuncId::QPopBack | SysFuncId::QPopFront => {
                let net = args
                    .first()
                    .and_then(|&a| match self.ir.exprs.get(a as usize) {
                        Some(Expr::Signal { net, word: None }) => Some(*net),
                        _ => None,
                    });
                match net.and_then(|n| self.ir.nets.get(n as usize).map(|nv| (n, nv))) {
                    Some((n, nv)) => {
                        self.nets.dyn_warn(
                            n,
                            "queue pop outside a direct blocking assign (X; not popped)",
                        );
                        Value::xs(nv.width.max(1), nv.signed)
                    }
                    None => Value::xs(32, false),
                }
            }
            // v6: assoc iteration methods WRITE their ref key argument — like
            // the pops they are legal only as the DIRECT rhs of a blocking
            // assign (statement-level intercept). Reaching THIS arm is an
            // unsupported placement: X status, no key write, loud.
            SysFuncId::AssocFirst
            | SysFuncId::AssocNext
            | SysFuncId::AssocLast
            | SysFuncId::AssocPrev => {
                if let Some(net) = args
                    .first()
                    .and_then(|&a| match self.ir.exprs.get(a as usize) {
                        Some(Expr::Signal { net, word: None }) => Some(*net),
                        _ => None,
                    })
                {
                    self.nets.dyn_warn(
                        net,
                        "assoc first/next/last/prev outside a direct blocking assign (X; key not written)",
                    );
                }
                Value::xs(32, true)
            }
            // v5 ⑤: `a.num()` — the entry count, same recipe as DynSize (the
            // reader's `dyn_size` covers assoc: num == size, IEEE §7.9.1).
            SysFuncId::AssocNum => {
                let net = args
                    .first()
                    .and_then(|&a| match self.ir.exprs.get(a as usize) {
                        Some(Expr::Signal { net, word: None }) => Some(*net),
                        _ => None,
                    });
                match net.and_then(|n| self.nets.dyn_size(n)) {
                    Some(n) => {
                        let mut v = Value::zeros(32, true);
                        v.val[0] = n.min(i32::MAX as u64);
                        v
                    }
                    None => Value::xs(32, true),
                }
            }
            // v5 ⑤: `a.exists(k)` — args = [handle, key]. PURE (a query, no
            // heap mutation), so unlike the pops it lives in the eval arm and
            // is VM-correct by construction. 1/0; X/Z key matches nothing.
            SysFuncId::AssocExists => {
                let net = args
                    .first()
                    .and_then(|&a| match self.ir.exprs.get(a as usize) {
                        Some(Expr::Signal { net, word: None }) => Some(*net),
                        _ => None,
                    });
                // v6: dispatch on the handle's key domain (string vs i64).
                let hit = net.and_then(|n| {
                    if self.nets.is_assoc_str(n) {
                        let key = args.get(1).and_then(|&k| self.assoc_str_key(k));
                        self.nets.assoc_str_exists(n, &key)
                    } else {
                        let key = args.get(1).and_then(|&k| self.assoc_key(k));
                        self.nets.assoc_exists(n, key)
                    }
                });
                match hit {
                    Some(b) => {
                        let mut v = Value::zeros(1, false);
                        v.val[0] = b as u64;
                        v
                    }
                    None => Value::xs(1, false),
                }
            }
            // ⓑ-breadth (v15/v17): array reduction methods (IEEE §7.12.3). PURE —
            // a read query that left-folds the element snapshot. Without a `with`
            // clause the result takes the element (handle) type and folds the raw
            // elements; WITH a `with (expr)` clause (args[1]) the result takes the
            // with-expr type and folds `expr(item)` per element. Empty → 0; x/z
            // propagates through the normal 4-state arithmetic/bitwise.
            SysFuncId::ArrSum
            | SysFuncId::ArrProduct
            | SysFuncId::ArrAnd
            | SysFuncId::ArrOr
            | SysFuncId::ArrXor => {
                let net = args
                    .first()
                    .and_then(|&a| match self.ir.exprs.get(a as usize) {
                        Some(Expr::Signal { net, word: None }) => Some(*net),
                        _ => None,
                    });
                let with_eid = args.get(1).copied();
                let elem_signed = net
                    .and_then(|n| self.ir.nets.get(n as usize))
                    .map(|nv| nv.signed)
                    .unwrap_or(true);
                // result type: with-expr width/sign, else element type
                let (w, signed) = match with_eid {
                    Some(weid) => {
                        let sw = self.wt.get(weid);
                        (sw.width.max(1), sw.signed)
                    }
                    None => net
                        .and_then(|n| self.ir.nets.get(n as usize))
                        .map(|nv| (nv.width.max(1), nv.signed))
                        .unwrap_or((32, true)),
                };
                let fold = |this: &Self, acc: &Value, e: &Value| -> Value {
                    match which {
                        SysFuncId::ArrSum => this.arith(BinOp::Add, acc, e),
                        SysFuncId::ArrProduct => this.arith(BinOp::Mul, acc, e),
                        SysFuncId::ArrAnd => this.bitwise(acc, e, and_w),
                        SysFuncId::ArrOr => this.bitwise(acc, e, or_w),
                        _ => this.bitwise(acc, e, xor_w),
                    }
                };
                match net.and_then(|n| self.nets.dyn_values(n)) {
                    Some(elems) if !elems.is_empty() => {
                        let saved = self.nets.swap_array_item(None);
                        let mut acc: Option<Value> = None;
                        for (i, e) in elems.iter().enumerate() {
                            let v = match with_eid {
                                Some(weid) => {
                                    self.nets.swap_array_item(Some((e.clone(), i as u64)));
                                    self.eval(weid)
                                }
                                None => {
                                    let mut x = e.clone();
                                    x.signed = elem_signed;
                                    x
                                }
                            };
                            acc = Some(match acc {
                                None => v,
                                Some(a) => fold(self, &a, &v),
                            });
                        }
                        self.nets.swap_array_item(saved);
                        acc.unwrap_or_else(|| Value::zeros(w, signed))
                            .resize_keep_sign(w, signed)
                    }
                    // empty array → element-type 0 (documented pin)
                    Some(_) => Value::zeros(w, signed),
                    // non-handle / string handle → defensive X
                    None => Value::xs(w, signed),
                }
            }
            SysFuncId::Time => {
                // $time: current time in the CALLING module's units, truncated to int
                // (now is global-precision ticks; divide by the module multiplier M).
                let m = self.time_mult.max(1);
                let mut v = Value::zeros(64, false);
                v.val[0] = self.now / m;
                v
            }
            SysFuncId::Realtime => {
                // $realtime: same as $time but keeping the sub-unit fraction.
                let m = self.time_mult.max(1) as f64;
                Value::from_f64(self.now as f64 / m)
            }
            SysFuncId::Rtoi => {
                // real → int, TRUNCATE toward zero. Result is a plain integer Value.
                let x = self.eval(args[0]).to_f64().unwrap_or(0.0);
                Value::from_i128(x.trunc() as i128, 32, true)
            }
            SysFuncId::Itor => {
                // int → real, exact convert.
                let i = self.eval(args[0]).to_i128_signed().unwrap_or(0);
                Value::from_f64(i as f64)
            }
            SysFuncId::RealToBits => {
                // real → 64-bit vector (raw IEEE bits). val[0] already holds
                // to_bits(); clear is_real so it reads as a plain 64-bit vector.
                let mut v = self.eval(args[0]);
                v.is_real = false;
                v.signed = false;
                v.width = 64;
                v
            }
            SysFuncId::BitsToReal => {
                // 64-bit vector → real. Same bits, set is_real. X/Z → NaN poison
                // (§6.2 "cannot convert X/Z to real") rather than a fabricated real.
                let src = self.eval(args[0]);
                if src.has_xz() {
                    return Value::from_f64(f64::NAN);
                }
                let mut v = src;
                v.is_real = true;
                v.signed = true;
                v.width = 64;
                v
            }
            SysFuncId::Signed => {
                let mut a = self.eval(args[0]);
                a.signed = true;
                a
            }
            SysFuncId::Unsigned => {
                let mut a = self.eval(args[0]);
                a.signed = false;
                a
            }
            SysFuncId::Clog2 => {
                // Exact at any width: $clog2(n) = highest set bit of (n-1) + 1.
                // Word-wise so a >64-bit argument is not truncated (P0-4).
                let a = self.eval(args[0]);
                if a.has_xz() {
                    return Value::xs(32, false);
                }
                let mut words: Vec<u64> = a.val.iter().copied().collect();
                let le_one = {
                    let w0 = words.first().copied().unwrap_or(0);
                    w0 <= 1 && words.iter().skip(1).all(|&w| w == 0)
                };
                let bits = if le_one {
                    0u64
                } else {
                    // n -= 1 (word-wise borrow), then locate the highest set bit.
                    for w in words.iter_mut() {
                        if *w > 0 {
                            *w -= 1;
                            break;
                        }
                        *w = u64::MAX;
                    }
                    let (k, top) = words
                        .iter()
                        .enumerate()
                        .rev()
                        .find(|(_, &w)| w != 0)
                        .map(|(k, &w)| (k as u64, w))
                        .unwrap_or((0, 0));
                    64 * k + (64 - top.leading_zeros() as u64)
                };
                let mut v = Value::zeros(32, false);
                v.val[0] = bits;
                v
            }
            // v7 bit-vector predicates (iverilog-pinned): x/z bits never count
            // as 1; the result is always KNOWN (a 1x1z operand gives co=2,
            // onehot=0 — not x). Word-parallel popcount over val & !unk.
            SysFuncId::CountOnes
            | SysFuncId::OneHot
            | SysFuncId::OneHot0
            | SysFuncId::IsUnknown => {
                let Some(&a0) = args.first() else {
                    // malformed arity (defensive — elaborate emits 1 arg)
                    return match which {
                        SysFuncId::CountOnes => Value::xs(32, true),
                        _ => Value::xs(1, false),
                    };
                };
                let a = self.eval(a0);
                let mut ones: u64 = 0;
                let mut unk_any = false;
                for k in 0..nwords(a.width).max(1) {
                    let v = a.val.get(k).copied().unwrap_or(0);
                    let u = a.unk.get(k).copied().unwrap_or(0);
                    ones += (v & !u).count_ones() as u64;
                    unk_any |= u != 0;
                }
                match which {
                    SysFuncId::CountOnes => {
                        let mut v = Value::zeros(32, true);
                        v.val[0] = ones;
                        v
                    }
                    SysFuncId::OneHot => Value::logic(ones == 1),
                    SysFuncId::OneHot0 => Value::logic(ones <= 1),
                    _ => Value::logic(unk_any),
                }
            }
            // v7 `$random` — no-arg form only (the seeded form is a
            // statement-level intercept that writes the ref seed back;
            // elaborate rejects any other seeded placement, so an arg here
            // is a hand-built IR — defensive X, no state advance).
            SysFuncId::Random => {
                if !args.is_empty() {
                    return Value::xs(32, true);
                }
                let mut s = self.rng.random.get();
                let r = crate::rng::annex_n_random(&mut s);
                self.rng.random.set(s);
                Value::from_i128(r as i128, 32, true)
            }
            // v7 `$urandom[(seed)]` — the optional seed is INPUT-only (IEEE
            // §18.13.1: not written back, unlike $random): it re-seeds the
            // generator, then the draw proceeds. X/Z seed = 0.
            SysFuncId::Urandom => {
                if let Some(&a0) = args.first() {
                    let seed = self.eval(a0).to_u64().unwrap_or(0);
                    self.rng.urandom.set(seed);
                }
                let mut st = self.rng.urandom.get();
                let r = crate::rng::splitmix_urandom(&mut st);
                self.rng.urandom.set(st);
                let mut v = Value::zeros(32, false);
                v.val[0] = r as u64;
                v
            }
            // v7 `$urandom_range(maxval[, minval])` — inclusive; swapped
            // bounds auto-correct (IEEE §18.13.3). X/Z bound → X result.
            SysFuncId::UrandomRange => {
                let bound = |i: usize| -> Option<u32> {
                    args.get(i)
                        .map(|&a| self.eval(a))
                        .filter(|v| !v.has_xz())
                        .and_then(|v| v.to_u64())
                        .map(|v| v as u32)
                };
                let Some(b0) = bound(0) else {
                    return Value::xs(32, false);
                };
                let b1 = if args.len() > 1 {
                    match bound(1) {
                        Some(b) => b,
                        None => return Value::xs(32, false),
                    }
                } else {
                    0
                };
                let (lo, hi) = if b0 <= b1 { (b0, b1) } else { (b1, b0) };
                let range = (hi - lo) as u64 + 1;
                let mut st = self.rng.urandom.get();
                let r = crate::rng::splitmix_urandom(&mut st);
                self.rng.urandom.set(st);
                let mut v = Value::zeros(32, false);
                v.val[0] = lo as u64 + (r as u64 % range);
                v
            }
            // v7 `$stime` — `$time` truncated to unsigned 32 bit (1364 §17.7.2).
            SysFuncId::Stime => {
                let m = self.time_mult.max(1);
                let mut v = Value::zeros(32, false);
                v.val[0] = (self.now / m) & 0xffff_ffff;
                v
            }
            // v7 `$test$plusargs(query)` — true iff some plusarg STARTS WITH
            // the query (iverilog-pinned prefix rule). The query is a string
            // LITERAL (elaborate enforces); a hand-built non-literal is X.
            SysFuncId::TestPlusargs => {
                let q = args.first().and_then(|&a| self.const_str_arg(a));
                match q {
                    Some(q) => {
                        let hit = self.plusargs.iter().any(|p| p.starts_with(&q));
                        Value::from_i128(hit as i128, 32, true)
                    }
                    None => Value::xs(32, true),
                }
            }
            // v7 P2-C string methods. args[0] = the handle's Signal (elaborate
            // contract); a malformed handle X-poisons, never panics. Methods
            // are PURE heap reads (putc is the one mutator = SysTask).
            SysFuncId::StrLen => {
                let b = self.handle_str_bytes(args.first());
                match b {
                    Some(b) => Value::from_i128(b.len() as i128, 32, true),
                    None => Value::xs(32, true),
                }
            }
            SysFuncId::StrGetC => {
                let b = self.handle_str_bytes(args.first());
                let i = args.get(1).and_then(|&a| self.eval(a).to_u64());
                match (b, i) {
                    // OOB index reads 0 (IEEE §6.16.2).
                    (Some(b), Some(i)) => {
                        let c = b.get(i as usize).copied().unwrap_or(0);
                        let mut v = Value::zeros(8, false);
                        v.val[0] = c as u64;
                        v
                    }
                    _ => Value::xs(8, false),
                }
            }
            SysFuncId::StrSubstr => {
                let b = self.handle_str_bytes(args.first());
                let i = args.get(1).and_then(|&a| self.eval(a).to_u64());
                let j = args.get(2).and_then(|&a| self.eval(a).to_u64());
                match (b, i, j) {
                    // inclusive [i..=j]; any invalid range = "" (IEEE §6.16.8).
                    (Some(b), Some(i), Some(j)) => {
                        let (i, j) = (i as usize, j as usize);
                        if i > j || j >= b.len() {
                            Value::from_str_bytes(&[])
                        } else {
                            Value::from_str_bytes(&b[i..=j])
                        }
                    }
                    _ => Value::from_str_bytes(&[]),
                }
            }
            SysFuncId::StrToUpper | SysFuncId::StrToLower => {
                let b = self.handle_str_bytes(args.first());
                match b {
                    Some(b) => {
                        let mapped: Vec<u8> = if matches!(which, SysFuncId::StrToUpper) {
                            b.iter().map(|c| c.to_ascii_uppercase()).collect()
                        } else {
                            b.iter().map(|c| c.to_ascii_lowercase()).collect()
                        };
                        Value::from_str_bytes(&mapped)
                    }
                    None => Value::from_str_bytes(&[]),
                }
            }
            // lexicographic compare of the two args' DENOTED byte strings
            // (§6.16 conversion: leading NULs strip) — backs both the
            // `.compare()` method and every string relational operator.
            SysFuncId::StrCmp => {
                let a = args.first().map(|&a| self.eval(a).to_str_bytes());
                let b = args.get(1).map(|&a| self.eval(a).to_str_bytes());
                match (a, b) {
                    (Some(a), Some(b)) => {
                        let r = match a.cmp(&b) {
                            std::cmp::Ordering::Less => -1i64,
                            std::cmp::Ordering::Equal => 0,
                            std::cmp::Ordering::Greater => 1,
                        };
                        Value::from_i128(r as i128, 32, true)
                    }
                    _ => Value::xs(32, true),
                }
            }
            // ⓑ-breadth (v18): string→number conversions (IEEE §6.16.9-13).
            // Parse the leading numeric prefix in the requested base; a leading
            // sign is honored for decimal (IEEE §6.16.9 — note iverilog 13 drops
            // it, its bug). Empty / non-numeric prefix → 0. Result truncates to
            // a 32-bit int.
            SysFuncId::StrAtoi
            | SysFuncId::StrAtohex
            | SysFuncId::StrAtooct
            | SysFuncId::StrAtobin => match self.handle_str_bytes(args.first()) {
                Some(b) => {
                    let (radix, signed) = match which {
                        SysFuncId::StrAtoi => (10, true),
                        SysFuncId::StrAtohex => (16, false),
                        SysFuncId::StrAtooct => (8, false),
                        _ => (2, false),
                    };
                    let n = parse_radix_prefix(&b, radix, signed);
                    Value::from_i128((n as i32) as i128, 32, true)
                }
                None => Value::xs(32, true),
            },
            SysFuncId::StrAtoreal => match self.handle_str_bytes(args.first()) {
                Some(b) => Value::from_f64(parse_real_prefix(&b)),
                None => Value::xs(64, true),
            },
            // v7 shape, features not wired yet (elaborate still rejects the
            // names): defensive X at each func's declared self-width.
            // `ValuePlusargs` here = unsupported placement (the legal direct-rhs
            // form is intercepted statement-level).
            SysFuncId::Fopen | SysFuncId::ValuePlusargs => Value::xs(32, true),
            // G1 (IEEE §6.16): an EXPRESSION-context `$sformatf` reaches eval ONLY
            // as elaborate's string-concat desugar — `$sformatf("%s%s…%s", p0, p1,
            // …)`, fmt = "%s"×N, args[1..] = the N concat parts. It renders to a
            // STRING-domain value so `$display("%s", {a,b})`, `{a,b}=="…"`, and
            // func/task args all see the concatenated bytes. The render matches the
            // statement-assign path's `%s` handler byte-for-byte (the oracle): a
            // string-literal Const → its decoded text; a string-domain value → its
            // exact bytes; any other integral value → its packed char bytes (§6.16).
            // (The statement-level `s = $sformatf(...)` form is intercepted before
            // eval; the kernel `format_args_str` honors arbitrary specs there.)
            SysFuncId::Sformatf => {
                let mut bytes: Vec<u8> = Vec::new();
                for &p in args.iter().skip(1) {
                    if let Some(s) = self.const_str_arg(p) {
                        bytes.extend_from_slice(s.as_bytes());
                    } else {
                        let v = self.eval(p);
                        if v.is_str {
                            bytes.extend_from_slice(&v.to_str_bytes());
                        } else {
                            bytes.extend_from_slice(
                                crate::builtins::fmt_packed_chars(&v).as_bytes(),
                            );
                        }
                    }
                }
                Value::from_str_bytes(&bytes)
            }
            // v9 shape-bump placeholders: the side-effecting file-read family,
            // $dist_*, and the $cast function form are all intercepted at the
            // statement level (direct rhs of a blocking assign) once ranks 5-6
            // wire them; in a non-intercepted eval context they yield X (the
            // same contract as `Fopen`/`ValuePlusargs`). elaborate emits none
            // of these yet, so this arm is dead until then.
            SysFuncId::Fgets
            | SysFuncId::Fscanf
            | SysFuncId::Sscanf
            | SysFuncId::Fread
            | SysFuncId::Feof
            | SysFuncId::Fgetc
            | SysFuncId::Ungetc
            | SysFuncId::DistUniform
            | SysFuncId::DistNormal
            | SysFuncId::DistExponential
            | SysFuncId::DistPoisson
            | SysFuncId::DistChiSquare
            | SysFuncId::DistT
            | SysFuncId::DistErlang
            | SysFuncId::Cast => Value::xs(32, true),
            // ── v19: N6 real-math (IEEE §20.8.2), computed via the vendored
            //   pure-Rust libm (3-OS byte-identical). An integral argument
            //   coerces to real (signed); a real argument is used as-is. Domain
            //   errors (e.g. $sqrt(-1), $acos(2), $ln(0)) propagate NaN/±inf
            //   exactly like C/iverilog. PURE — no seed/heap mutation. ──
            SysFuncId::Ln
            | SysFuncId::Log10
            | SysFuncId::Exp
            | SysFuncId::Sqrt
            | SysFuncId::Pow
            | SysFuncId::Floor
            | SysFuncId::Ceil
            | SysFuncId::Sin
            | SysFuncId::Cos
            | SysFuncId::Tan
            | SysFuncId::Asin
            | SysFuncId::Acos
            | SysFuncId::Atan
            | SysFuncId::Atan2
            | SysFuncId::Hypot
            | SysFuncId::Sinh
            | SysFuncId::Cosh
            | SysFuncId::Tanh
            | SysFuncId::Asinh
            | SysFuncId::Acosh
            | SysFuncId::Atanh => {
                let real_arg = |i: usize| -> f64 {
                    match args.get(i) {
                        Some(&a) => {
                            let v = self.eval(a);
                            if v.is_real {
                                v.to_f64().unwrap_or(0.0)
                            } else {
                                // integral operand → real (signed); X/Z → 0.0.
                                v.to_i128_signed().unwrap_or(0) as f64
                            }
                        }
                        None => 0.0,
                    }
                };
                let x = real_arg(0);
                let r = match which {
                    SysFuncId::Ln => libm::log(x),
                    SysFuncId::Log10 => libm::log10(x),
                    SysFuncId::Exp => libm::exp(x),
                    SysFuncId::Sqrt => libm::sqrt(x),
                    SysFuncId::Pow => libm::pow(x, real_arg(1)),
                    SysFuncId::Floor => libm::floor(x),
                    SysFuncId::Ceil => libm::ceil(x),
                    SysFuncId::Sin => libm::sin(x),
                    SysFuncId::Cos => libm::cos(x),
                    SysFuncId::Tan => libm::tan(x),
                    SysFuncId::Asin => libm::asin(x),
                    SysFuncId::Acos => libm::acos(x),
                    SysFuncId::Atan => libm::atan(x),
                    SysFuncId::Atan2 => libm::atan2(x, real_arg(1)),
                    SysFuncId::Hypot => libm::hypot(x, real_arg(1)),
                    SysFuncId::Sinh => libm::sinh(x),
                    SysFuncId::Cosh => libm::cosh(x),
                    SysFuncId::Tanh => libm::tanh(x),
                    SysFuncId::Asinh => libm::asinh(x),
                    SysFuncId::Acosh => libm::acosh(x),
                    SysFuncId::Atanh => libm::atanh(x),
                    _ => unreachable!("real-math arm gates on the same id set"),
                };
                Value::from_f64(r)
            }
        }
    }

    /// `$signed`/`$unsigned`/`$time`/`$clog2` in context: cast preserves width
    /// but flips sign; $time/$clog2 produce a fixed-width value then extend.
    fn eval_sysfunc_ctx(&self, which: SysFuncId, args: &[u32], w: u32, eff_signed: bool) -> Value {
        match which {
            SysFuncId::Signed => {
                // operand at its OWN self width. `$signed` re-stamps it signed, but
                // the EXTENSION FILL is governed by `eff_signed` (= self-signed AND
                // ctx_signed), NOT the unconditional cast: under the global-unsigned
                // rule (§5.5.1) an unsigned sibling makes the whole region unsigned,
                // so `$signed(x)` must ZERO-extend there, not sign-extend. Setting
                // `.signed = eff_signed` BEFORE the resize makes the fill policy
                // unambiguously flag-driven.
                let mut a = self.eval(args[0]);
                a.signed = eff_signed;
                a.resize_keep_sign(w, eff_signed)
            }
            SysFuncId::Unsigned => {
                let mut a = self.eval(args[0]);
                a.signed = false;
                a.resize_keep_sign(w, false) // unsigned cast → zero-extend
            }
            // $time/$realtime (64-bit) and $clog2 (32-bit): natural value, then
            // resize to context (zero/sign per eff_signed).
            _ => self
                .eval_sysfunc(which, args)
                .resize_keep_sign(w, eff_signed),
        }
    }

    // ── helpers ────────────────────────────────────────────────────────────

    fn truthiness(&self, a: &Value) -> Tri {
        // A real is logically true iff it is != 0.0 (IEEE 1364: `-0.0 == 0.0`,
        // so both signed zeros are FALSE; NaN != 0.0 → truthy). Reinterpreting a
        // real's f64 bits as a 4-state vector would wrongly read `-0.0`
        // (sign bit set, value zero) as true in `if`/`!`/ternary/`&&`/`||`.
        if a.is_real {
            return if a.to_f64().unwrap_or(0.0) != 0.0 {
                Tri::True
            } else {
                Tri::False
            };
        }
        let mut any_unknown = false;
        for i in 0..a.width {
            let (v, u) = a.get_vu(i);
            if u == 0 && v == 1 {
                return Tri::True;
            }
            if u != 0 {
                any_unknown = true;
            }
        }
        if any_unknown {
            Tri::Unknown
        } else {
            Tri::False
        }
    }
}

enum Tri {
    True,
    False,
    Unknown,
}

fn ipow_signed(base: i128, exp: i128) -> u128 {
    if exp < 0 {
        // negative exponent on integers → 0 (except base==1 → 1, base==-1 → ±1)
        return match base {
            1 => 1,
            -1 => {
                if exp % 2 == 0 {
                    1
                } else {
                    (-1i128) as u128
                }
            }
            _ => 0,
        };
    }
    let mut acc: i128 = 1;
    let mut e = exp;
    let mut b = base;
    while e > 0 {
        if e & 1 == 1 {
            acc = acc.wrapping_mul(b);
        }
        b = b.wrapping_mul(b);
        e >>= 1;
    }
    acc as u128
}

// ── multi-word kernels (Phase-1.x ⑥) — all operate on little-endian u64
//    word vectors of equal length; callers mask to the target width. ──────

fn mw_add(a: &[u64], b: &[u64]) -> Vec<u64> {
    let mut out = vec![0u64; a.len()];
    let mut carry = 0u64;
    for k in 0..a.len() {
        let (s1, c1) = a[k].overflowing_add(b.get(k).copied().unwrap_or(0));
        let (s2, c2) = s1.overflowing_add(carry);
        out[k] = s2;
        carry = (c1 as u64) + (c2 as u64);
    }
    out
}

/// In-place `dest += b` on the word grid; bit-identical to
/// `dest = mw_add(&dest, b)` but reuses `dest`'s allocation (hot in the
/// restoring-division loop, where `b` is the pre-negated divisor).
fn mw_add_inplace(dest: &mut [u64], b: &[u64]) {
    let mut carry = 0u64;
    for (k, d) in dest.iter_mut().enumerate() {
        let (s1, c1) = d.overflowing_add(b.get(k).copied().unwrap_or(0));
        let (s2, c2) = s1.overflowing_add(carry);
        *d = s2;
        carry = (c1 as u64) + (c2 as u64);
    }
}

/// Two's complement on the word grid (`!a + 1`); caller masks to width.
pub(crate) fn mw_neg(a: &[u64]) -> Vec<u64> {
    let mut out = vec![0u64; a.len()];
    let mut carry = 1u64;
    for k in 0..a.len() {
        let (s, c) = (!a[k]).overflowing_add(carry);
        out[k] = s;
        carry = c as u64;
    }
    out
}

pub(crate) fn mw_mask(mut a: Vec<u64>, w: u32) -> Vec<u64> {
    let n = nwords(w).max(1);
    a.truncate(n);
    a.resize(n, 0);
    let top = w - 64 * (n as u32 - 1);
    a[n - 1] &= low_mask(top);
    a
}

fn mw_is_zero(a: &[u64]) -> bool {
    a.iter().all(|&x| x == 0)
}

fn mw_one(n: usize) -> Vec<u64> {
    let mut v = vec![0u64; n];
    v[0] = 1;
    v
}

/// School multiplication, LOW `n` words (mod 2^(64n)).
fn mw_mul(a: &[u64], b: &[u64], n: usize) -> Vec<u64> {
    let mut out = vec![0u64; n];
    for i in 0..n.min(a.len()) {
        if a[i] == 0 {
            continue;
        }
        let mut carry = 0u128;
        for j in 0..n - i {
            let bj = b.get(j).copied().unwrap_or(0);
            let cur = (a[i] as u128) * (bj as u128) + (out[i + j] as u128) + carry;
            out[i + j] = cur as u64;
            carry = cur >> 64;
        }
    }
    out
}

fn mw_cmp(a: &[u64], b: &[u64]) -> std::cmp::Ordering {
    for k in (0..a.len().max(b.len())).rev() {
        let av = a.get(k).copied().unwrap_or(0);
        let bv = b.get(k).copied().unwrap_or(0);
        match av.cmp(&bv) {
            std::cmp::Ordering::Equal => continue,
            o => return o,
        }
    }
    std::cmp::Ordering::Equal
}

/// Unsigned divmod; `b != 0` (caller-gated). One-word divisors take the O(n)
/// short path; otherwise classic restoring long division over the dividend
/// bits (O(bits·n) word ops).
fn mw_divmod(a: &[u64], b: &[u64]) -> (Vec<u64>, Vec<u64>) {
    let n = a.len();
    if b.iter().skip(1).all(|&x| x == 0) {
        let d = b[0] as u128;
        let mut q = vec![0u64; n];
        let mut rem = 0u128;
        for k in (0..n).rev() {
            let cur = (rem << 64) | a[k] as u128;
            q[k] = (cur / d) as u64;
            rem = cur % d;
        }
        let mut r = vec![0u64; n];
        r[0] = rem as u64;
        return (q, r);
    }
    // rem gets one extra word so the shift-in never clips.
    let mut rem = vec![0u64; n + 1];
    let mut bx = b.to_vec();
    bx.push(0);
    // `bx` is loop-invariant, so its two's complement is too — negate once and
    // subtract in place (was `mw_neg(&bx)` + a fresh `mw_add` Vec every bit).
    let neg_bx = mw_neg(&bx);
    let mut q = vec![0u64; n];
    for i in (0..n as u32 * 64).rev() {
        // rem = (rem << 1) | bit i of a
        let mut carry = (a[(i / 64) as usize] >> (i % 64)) & 1;
        for word in rem.iter_mut() {
            let top = *word >> 63;
            *word = (*word << 1) | carry;
            carry = top;
        }
        if mw_cmp(&rem, &bx) != std::cmp::Ordering::Less {
            mw_add_inplace(&mut rem, &neg_bx);
            q[(i / 64) as usize] |= 1 << (i % 64);
        }
    }
    rem.truncate(n);
    (q, rem)
}

/// Exact decimal rendering of an arbitrary-width unsigned word vector:
/// repeated short division by 10^19 (the largest power of ten in a u64),
/// emitting 19-digit chunks. Phase-1.x ⑥ — `%d` used to truncate past 128.
pub(crate) fn mw_decimal(words: &[u64]) -> String {
    const D: u128 = 10_000_000_000_000_000_000; // 10^19
    if mw_is_zero(words) {
        return "0".to_string();
    }
    let mut w = words.to_vec();
    let mut chunks: Vec<u64> = Vec::new();
    while !mw_is_zero(&w) {
        let mut rem: u128 = 0;
        for k in (0..w.len()).rev() {
            let cur = (rem << 64) | w[k] as u128;
            w[k] = (cur / D) as u64;
            rem = cur % D;
        }
        chunks.push(rem as u64);
    }
    let mut out = chunks.pop().unwrap().to_string();
    for c in chunks.into_iter().rev() {
        out.push_str(&format!("{c:019}"));
    }
    out
}

/// Square-multiply power mod 2^w (exponent ≥ 0, X-free).
fn mw_pow(base: &[u64], exp: &[u64], w: u32) -> Vec<u64> {
    let n = base.len();
    let top = match (0..n * 64)
        .rev()
        .find(|&i| (exp[i / 64] >> (i % 64)) & 1 == 1)
    {
        None => return mw_one(n), // exp == 0
        Some(t) => t,
    };
    let mut acc = mw_one(n);
    let mut sq = base.to_vec();
    for i in 0..=top {
        if (exp[i / 64] >> (i % 64)) & 1 == 1 {
            acc = mw_mask(mw_mul(&acc, &sq, n), w);
        }
        if i < top {
            sq = mw_mask(mw_mul(&sq, &sq, n), w);
        }
    }
    acc
}
