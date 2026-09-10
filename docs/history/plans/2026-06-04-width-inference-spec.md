# Width-Inference Spec — IEEE 1364-2005 Context-Determined Sizing (engine-only)

**Status:** implementation-ready. **Scope:** entirely inside `crates/sim-engine`. The `sim-ir` IR is **FROZEN** — this spec adds **zero** fields to any `sim-ir` type and reads `SimIr`/`NetVar`/`ConstVal`/`Expr` verbatim. No new diag codes (bijection gate untouched — see §11).

**Date:** 2026-06-04
**Author:** engine architect (vitamin)
**Target crate:** `crates/sim-engine` only
**MSRV:** 1.82, edition 2021, `--locked`

---

## 0. Problem statement (verbatim from research)

The engine has **no context-determined width**. `eval.rs:338` documents that elaborate defers expression sizing. Every binary op self-determines its width as `max(operand widths)`. Consequence: a continuous/procedural assignment whose RHS computes a carry/overflow bit (`sum = a + b` with `sum` wider than `a`,`b`) **drops the carry**, because the add is evaluated at the 4-bit operand max and only the *result* is widened on the write. This violates IEEE 1364-2005 §5.4.1 (Table 5-22) / §5.5.

This spec introduces a **self-width side table** computed at engine init and a **context-propagating evaluator** `eval_ctx(eid, ctx_width, ctx_signed)` so that the LHS width flows DOWN into the RHS's context-determined operands BEFORE the operation — without touching the frozen IR.

---

## 1. Arena ordering — VERIFIED post-order (single forward pass is sound)

**Claim under test:** in `SimIr.exprs`, every child ExprId is strictly **less than** its parent ExprId (topological / post-order), so a single forward scan `0..exprs.len()` can fill a side table where each node only reads already-filled children.

**Verification (elaborate is the sole producer of `exprs`):**

- `crates/elaborate/src/lib.rs:1892` — `fn push_expr(&mut self, e) -> u32 { let id = self.exprs.len(); self.exprs.push(e); id }` is **THE deterministic expr append point** (only place that grows the arena; documented choke point at `lib.rs:26`).
- `fn lower_expr` (`lib.rs:1271`) recurses into **every** child via `self.lower_expr(child)` (or `const_u32_expr`, both of which `push_expr`) **before** calling `push_expr` for the parent. Confirmed per variant:
  - `Unary` (1298–1302): `operand = lower_expr(...)` then push.
  - `Binary` (1304–1311): `lhs`, then `rhs`, then push (comment "POST-ORDER: lhs, then rhs, then self").
  - `Ternary` (1318–1325): cond, then_e, else_e, then push.
  - `Select`/`BitSelect`/`PartSelect`/`IndexedPart` (1329–1384): `base`, `offset`, `width` all lowered (and `width` for PartConst is a synthesized `Add(Sub(msb,lsb),1)` tree — inner `Sub` pushed at 1971, outer `Add` at 1976; the stored `width` ExprId is the outer `Add`) **before** the `Select` push.
  - `Concat` (1387–1390): all `parts` mapped through `lower_expr` then push.
  - `Replicate` (1391–1400): `count`, the inner `Concat` (`value`), then `Replicate` push.
  - `SysFunc`/`SysCall` (1403–1418): all `args` lowered then push.
  - `Const`/`Signal` leaves (1274–1293): push with no child ExprIds (children are const/net indices, different arenas).
  - `Ident` inline-subst (1282–1290): may **return an existing lower-indexed ExprId** without pushing — still a back-reference (≤ current len), invariant preserved.
  - `Paren`/`MinTypMax` (1422–1423): transparent, return the inner ExprId (lower index).

**Conclusion:** children always carry **smaller** indices than parents (or equal-class back-refs that are already filled). A **single forward pass** suffices. **No memoized recursion needed.**

**Defensive guard (cheap, keeps us honest):** the self-width pass asserts each child index `< self_idx` as it reads it. If a future elaborate change ever violates post-order, this fires deterministically instead of reading an uninitialized slot. The assert is `debug_assert!` in the hot path plus a hard `assert!` only on the first violation discovered via a `child < i` check folded into a helper (`§3.2`). If the guard is undesirable as a hard panic, the fallback is memoized recursion with a `Vec<State>{Unseen,InProgress,Done}` cycle detector — **specified in §3.5 as the contingency**, not the default.

---

## 2. Data model — the self-width side table

```rust
// crates/sim-engine/src/width.rs  (NEW FILE)

//! Engine-side IEEE 1364-2005 context-determined width inference.
//!
//! Builds a side table `Vec<SelfWidth>` indexed by ExprId, parallel to
//! `SimIr.exprs`, computed once at `SimState::new`. The frozen sim-ir is read
//! verbatim; this table lives ENTIRELY in engine state. It encodes each expr's
//! self-determined (bottom-up) width and signedness per §5.4.1 / §5.5.

use sim_ir::{BinOp, ConstRepr, Expr, SelKind, SimIr, SysFuncId, UnOp};

/// Self-determined sizing of one expression node (IEEE §5.4.1 / §5.5).
/// `width` is the bottom-up self-width; `signed` is the self-signedness
/// (the both-signed rule already folded in for context-determined operators).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelfWidth {
    pub width: u32,
    pub signed: bool,
}

/// The whole side table, one entry per `SimIr.exprs[i]`.
pub(crate) struct WidthTable {
    sw: Vec<SelfWidth>,
}

impl WidthTable {
    #[inline]
    pub(crate) fn get(&self, eid: u32) -> SelfWidth {
        self.sw[eid as usize]
    }
    #[inline]
    pub(crate) fn width(&self, eid: u32) -> u32 {
        self.sw[eid as usize].width
    }
    #[inline]
    pub(crate) fn signed(&self, eid: u32) -> bool {
        self.sw[eid as usize].signed
    }
    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.sw.len()
    }
}
```

**Width clamping:** all widths are `u32`. To stay aligned with existing engine caps (`shift_left` caps growth at 4096; `Value` is word-vec backed), the table clamps any computed width with a saturating helper and a hard ceiling `WIDTH_MAX = 1 << 24` (16 Mibits — far beyond any RTL vector, well under `u32` overflow on `count * width` for replication). Saturation never *narrows* a legitimately-needed width; it only prevents `u32` multiply overflow producing a tiny wrapped width (a correctness trap). All `+`/`*` below use `saturating_add` / `saturating_mul` then `.min(WIDTH_MAX)`.

```rust
const WIDTH_MAX: u32 = 1 << 24;
#[inline]
// `w` is u32 so it is already ≥ 0; do NOT add `.max(0)` — that is a no-op that
// trips `clippy::unnecessary_min_or_max` under `-D warnings`. A floor of 1 is
// applied separately by callers that need it (`.max(1)`), not here.
fn clamp_w(w: u32) -> u32 { w.min(WIDTH_MAX) }
#[inline]
fn add_w(a: u32, b: u32) -> u32 { clamp_w(a.saturating_add(b)) }
#[inline]
fn mul_w(a: u32, b: u32) -> u32 { clamp_w(a.saturating_mul(b)) }
```

---

## 3. THE SELF-WIDTH PASS

### 3.1 Entry point

```rust
impl WidthTable {
    /// Build the self-width table by a single forward pass over `ir.exprs`.
    /// PRECONDITION (verified §1): every child ExprId < its parent ExprId, so a
    /// forward scan reads only already-filled entries.
    pub(crate) fn build(ir: &SimIr) -> WidthTable {
        let n = ir.exprs.len();
        let mut sw: Vec<SelfWidth> = Vec::with_capacity(n);
        for i in 0..n {
            let s = Self::self_width_of(ir, &sw, i as u32);
            debug_assert_eq!(sw.len(), i, "forward pass invariant");
            sw.push(s);
        }
        WidthTable { sw }
    }
}
```

`sw` holds entries `0..i` when computing entry `i`; reads of children (all `< i`) index into `sw`. The `debug_assert_eq!` pins the forward invariant.

### 3.2 Child reader with the post-order guard

```rust
/// Read an already-computed child's self-width. `child` MUST be < `parent`
/// (post-order arena, §1). The guard converts any future ordering regression
/// into a deterministic panic instead of reading a default/garbage slot.
#[inline]
fn child(sw: &[SelfWidth], parent: u32, child: u32) -> SelfWidth {
    assert!(
        (child as usize) < sw.len() && child < parent,
        "width pass: child {child} not yet computed for parent {parent} \
         (arena not post-order — see width.rs §1)"
    );
    sw[child as usize]
}
```

### 3.3 Per-variant self-width formulas (EXHAUSTIVE)

```rust
fn self_width_of(ir: &SimIr, sw: &[SelfWidth], i: u32) -> SelfWidth {
    match &ir.exprs[i as usize] {

        // ── leaves ──────────────────────────────────────────────────────
        Expr::Const { val } => {
            let c = &ir.consts[*val as usize];
            // A const is signed ONLY when repr==Numeric AND its signed flag set
            // (string consts never signed) — mirrors eval_const (eval.rs:67).
            let signed = matches!(c.repr, ConstRepr::Numeric) && c.signed;
            SelfWidth { width: clamp_w(c.width.max(1)), signed }
        }
        Expr::Signal { net, .. } => {
            let nv = &ir.nets[*net as usize];
            // `word` selects an ARRAY element; element width == NetVar.width.
            SelfWidth { width: clamp_w(nv.width.max(1)), signed: nv.signed }
        }

        // ── select: bit=1 (UNSIGNED), part=`width` operand (UNSIGNED) ─────
        // IEEE §5.4.1: bit-select and part-select results are ALWAYS unsigned.
        Expr::Select { width, kind, .. } => {
            let w = match kind {
                SelKind::Bit => 1,
                // `width` is an EXPR INDEX (frozen IR). v1 elaborate emits a
                // constant width tree for PartConst/PartIdxUp/PartIdxDown; we
                // resolve it via the const-fold helper (§3.4). If it does not
                // const-fold, fall back to the child's self-width (defensive —
                // engine-side part width is the selected range, not the index
                // expr's own width); eval_select still clamps at runtime.
                _ => const_u32_of_expr(ir, *width).unwrap_or(1),
            };
            SelfWidth { width: clamp_w(w.max(1)), signed: false }
        }

        // ── concat: SUM of part self-widths, ALWAYS UNSIGNED, self-determined ─
        Expr::Concat { parts } => {
            let mut total = 0u32;
            for &p in parts {
                total = add_w(total, child(sw, i, p).width);
            }
            SelfWidth { width: clamp_w(total.max(1)), signed: false }
        }

        // ── replicate: count * width(value), ALWAYS UNSIGNED, self-determined ─
        Expr::Replicate { count, value } => {
            let n = const_u32_of_expr(ir, *count).unwrap_or(0);
            let vw = child(sw, i, *value).width;
            SelfWidth { width: clamp_w(mul_w(n, vw).max(1)), signed: false }
        }

        // ── unary ─────────────────────────────────────────────────────────
        Expr::Unary { op, operand } => {
            let o = child(sw, i, *operand);
            match op {
                // context-determined unary: width = operand width, sign = operand
                UnOp::Plus | UnOp::Minus | UnOp::BitNot => {
                    SelfWidth { width: o.width.max(1), signed: o.signed }
                }
                // reductions + logical-not: 1-bit, UNSIGNED, operand self-det
                UnOp::LogNot
                | UnOp::RedAnd | UnOp::RedNand
                | UnOp::RedOr  | UnOp::RedNor
                | UnOp::RedXor | UnOp::RedXnor => {
                    SelfWidth { width: 1, signed: false }
                }
            }
        }

        // ── binary ────────────────────────────────────────────────────────
        Expr::Binary { op, lhs, rhs } => {
            let l = child(sw, i, *lhs);
            let r = child(sw, i, *rhs);
            match op {
                // arithmetic + bitwise: max(L,R), signed iff BOTH signed
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div
                | BinOp::Mod
                | BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::BitXnor => {
                    SelfWidth {
                        width: clamp_w(l.width.max(r.width).max(1)),
                        signed: l.signed && r.signed,
                    }
                }
                // power: width = max(L,R) (IEEE Table 5-22), but the EXPONENT is
                // SELF-DETERMINED (like a shift amount), so it does NOT participate
                // in the both-signed fold. `**` is signed iff the BASE is signed —
                // an unsigned exponent must NOT demote a signed base to unsigned.
                BinOp::Pow => {
                    SelfWidth {
                        width: clamp_w(l.width.max(r.width).max(1)),
                        signed: l.signed,                 // BASE sign only
                    }
                }
                // comparisons / case-eq / logical: 1-bit, UNSIGNED
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
                | BinOp::Eq | BinOp::Ne | BinOp::CaseEq | BinOp::CaseNe
                | BinOp::LogAnd | BinOp::LogOr => {
                    SelfWidth { width: 1, signed: false }
                }
                // shifts: width = LEFT operand width, sign follows LEFT.
                // RHS (amount) is SELF-DETERMINED — does not affect this width.
                BinOp::Shl | BinOp::Shr | BinOp::AShl | BinOp::AShr => {
                    SelfWidth { width: l.width.max(1), signed: l.signed }
                }
            }
        }

        // ── ternary: max(then,else), signed iff BOTH branches signed ───────
        Expr::Ternary { then_e, else_e, .. } => {
            let t = child(sw, i, *then_e);
            let e = child(sw, i, *else_e);
            SelfWidth {
                width: clamp_w(t.width.max(e.width).max(1)),
                signed: t.signed && e.signed,
            }
        }

        // ── system functions (§6) ──────────────────────────────────────────
        Expr::SysFunc { which, args } => match which {
            // $time / $realtime: 64-bit unsigned (realtime real modeled as 64-bit).
            SysFuncId::Time | SysFuncId::Realtime => SelfWidth { width: 64, signed: false },
            // $signed / $unsigned: PRESERVE operand width, flip sign attribute.
            SysFuncId::Signed => {
                let w = args.first().map(|&a| child(sw, i, a).width).unwrap_or(1);
                SelfWidth { width: w.max(1), signed: true }
            }
            SysFuncId::Unsigned => {
                let w = args.first().map(|&a| child(sw, i, a).width).unwrap_or(1);
                SelfWidth { width: w.max(1), signed: false }
            }
            // $clog2: integer return → 32-bit signed `integer` convention.
            SysFuncId::Clog2 => SelfWidth { width: 32, signed: true },
        },

        // ── user function call: return-net width (resolve via FuncDef) ──────
        // v1 eval returns 1-bit X for Call (eval.rs:53). Mirror that here so the
        // table is consistent with eval until user-functions land in v2.
        // NOTE: elaborate v1 NEVER actually emits `ir::Expr::Call` — `inline_function`
        // (elaborate/lib.rs:1636) always returns an existing/lowered ExprId or a
        // `Const` placeholder, even on error paths. This arm is therefore
        // defensive/unreachable in practice; it exists only for exhaustive-match
        // safety on the frozen enum variant (mirrors eval.rs:53).
        Expr::Call { .. } => SelfWidth { width: 1, signed: false },
    }
}
```

### 3.4 Const-fold helper for `Select.width`, `Replicate.count`

`Select.width`, `Replicate.count` are **ExprId**s (frozen IR), not literals. v1 elaborate emits a constant tree for these (a `Const` expr, or for `PartConst` the synthesized **`Add(Sub(msb,lsb), 1)`** tree — `width_from_msb_lsb_checked` pushes the inner `Sub` at `elaborate/lib.rs:1971` and the outer `Add(.., 1)` at `elaborate/lib.rs:1976`; the `width` ExprId stored on the `Select` is the OUTER `Add` node). We need their compile-time value to size the table:

```rust
/// Resolve an expr to a compile-time u32 if it is a literal const (the v1
/// elaborate guarantee for part-select width and replicate count). Returns
/// None for non-const trees (e.g. a runtime offset, which never feeds width).
/// NOTE: this is a SHALLOW fold over exactly the trees elaborate synthesizes:
///   - direct `Const` (replicate count, PartIdxUp/Down width: lib.rs:1373);
///   - `Add(lhs, rhs)` — the OUTER node of `[msb:lsb]` width `Add(Sub(msb,lsb),1)`;
///   - `Sub(lhs, rhs)` — the INNER `msb - lsb` node.
/// `Mul`/other shapes → None (caller falls back to width 1; eval clamps at run).
fn const_u32_of_expr(ir: &SimIr, eid: u32) -> Option<u32> {
    match &ir.exprs[eid as usize] {
        Expr::Const { val } => {
            let c = &ir.consts[*val as usize];
            // value-plane (`bits.val: Vec<u64>`) word0, no X/Z. Reject if any
            // unknown bit set. Anti-wrap (§10): a count/width above u32::MAX
            // would be silently truncated by `as u32`, so CLAMP to WIDTH_MAX
            // whenever any high word0 bits (≥ bit 32) OR any word ≥ 1 are nonzero,
            // rather than wrap/truncate to a bogus small width.
            if c.bits.unk.iter().any(|&u| u != 0) { return None; }
            let word0 = c.bits.val.first().copied().unwrap_or(0);
            let high_words_set = c.bits.val.iter().skip(1).any(|&v| v != 0);
            if high_words_set || word0 > u32::MAX as u64 {
                return Some(WIDTH_MAX);
            }
            Some((word0 as u32).min(WIDTH_MAX))
        }
        // OUTER node of the `[msb:lsb]` width tree: `(msb - lsb) + 1`.
        Expr::Binary { op: BinOp::Add, lhs, rhs } => {
            let a = const_u32_of_expr(ir, *lhs)?;
            let b = const_u32_of_expr(ir, *rhs)?;
            Some(clamp_w(a.saturating_add(b)))
        }
        // INNER node of the `[msb:lsb]` width tree: `msb - lsb`.
        Expr::Binary { op: BinOp::Sub, lhs, rhs } => {
            let a = const_u32_of_expr(ir, *lhs)?;
            let b = const_u32_of_expr(ir, *rhs)?;
            Some(a.saturating_sub(b))
        }
        _ => None,
    }
}
```

This exactly mirrors `width_from_msb_lsb_checked` (`elaborate/lib.rs:1955–1981`), which builds `Add(Sub(msb, lsb), 1)`: the `Sub` arm folds `msb - lsb`, the `Add` arm adds the `+1` literal, yielding the true range width. For `PartIdxUp`/`PartIdxDown` the `width` operand is a direct const (`elaborate/lib.rs:1373`). For `Bit` we ignore `width` and use 1. `msb_id`/`lsb_id` are arbitrary lowered expr trees; if either is non-constant the fold returns `None` and the table falls back to width 1 (§10 item 2 — only reachable for runtime-variable bounds, which v1 elaborate does not emit for `PartConst`).

### 3.5 Contingency: memoized recursion (only if §1 regresses)

If a future elaborate change ever breaks post-order (the §3.2 guard panics), swap `build` for:

```rust
pub(crate) fn build(ir: &SimIr) -> WidthTable {
    enum S { Unseen, InProgress, Done(SelfWidth) }
    let mut st = vec![S::Unseen; ir.exprs.len()];
    fn go(ir, st, i) -> SelfWidth {
        match st[i] { Done(s)=>return s, InProgress=>panic!("expr cycle"), Unseen=>{} }
        st[i] = InProgress;
        let s = self_width_of_rec(ir, st, i, &mut |c| go(ir, st, c));
        st[i] = Done(s); s
    }
    /* ... fill all via go(ir,&mut st,i) ... */
}
```

with a `Vec<S>` `Unseen/InProgress/Done` cycle detector (the IR is a DAG; `InProgress` re-entry ⇒ cycle ⇒ panic, never silent). **This is NOT the default** — the single forward pass is proven sound by §1.

---

## 4. THE CONTEXT EVALUATOR `eval_ctx`

### 4.1 Threading the table into `EvalCtx`

`EvalCtx` gains a borrow of the table. Construction sites (`sched.rs:367`, `sched.rs:376`) gain the new field.

```rust
// eval.rs
pub struct EvalCtx<'a, N: NetReader> {
    pub ir: &'a SimIr,
    pub nets: &'a N,
    pub now: u64,
    pub wt: &'a crate::width::WidthTable,   // NEW
}
```

### 4.2 `eval` becomes a thin wrapper over `eval_ctx`

The current public `eval(eid)` keeps its signature and semantics for **self-determined top-level evaluation** (truthiness, systask args, ternary cond, shift amount): it evaluates a node at its OWN self-width.

```rust
impl<'a, N: NetReader> EvalCtx<'a, N> {
    /// Self-determined eval: size the node to its own self-width. Unchanged
    /// public surface; used by control-flow truthiness and systask args.
    pub fn eval(&self, eid: u32) -> Value {
        let sw = self.wt.get(eid);
        self.eval_ctx(eid, sw.width, sw.signed)
    }
```

**MANDATORY DELETIONS (no-dead-code gate).** Once `eval` delegates to `eval_ctx`, the three private dispatch methods `eval_unary` (`eval.rs:73`), `eval_binary` (`eval.rs:135`), and `eval_ternary` (`eval.rs:386`) have **ZERO callers** — their logic is fully subsumed by `eval_ctx`'s inlined unary/binary/ternary arms (`eval_binary_ctx`, `eval_unary_self`, the inline `Minus`/`Plus`/`BitNot` arms, and `merge_x`). The workspace builds under `cargo clippy --workspace --all-targets --locked -- -D warnings` and sim-engine carries **no** crate-level `#![allow(dead_code)]`, so leaving them in **fails the lint gate** (`dead_code` denied). **The implementer MUST delete all three.** The per-bit/per-op leaf helpers they used (`negate`, `bitwise`, `arith`, `relational`, `log_eq`, `case_eq`, `log_and`, `log_or`, `shift_left`, `shift_right`, `reduce`, `reduce_not`, and the `and1`/`or1`/`xor1`/`xnor1`/`not1` primitives) all retain callers via `eval_ctx`/`eval_binary_ctx`/`eval_unary_self`, so they stay. (The old `eval_ternary` unknown-merge body is preserved verbatim as `merge_x`, §4.6 — so no logic is lost.) See §12 manifest for the explicit deletion line.

### 4.3 `eval_ctx` — the context-propagating core

```rust
    /// Evaluate `eid` in a context of at least `ctx_width` bits with context
    /// signedness `ctx_signed`. Returns a Value of width `max(self_width, ctx_width)`.
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
                let base = self.nets.read_net(*net, *word);
                base.resize_keep_sign(w, eff_signed)
            }

            // ── unary ──────────────────────────────────────────────────────
            Expr::Unary { op, operand } => match op {
                // context-determined unary: propagate (w, eff_signed) into operand,
                // operate at w, result already w-wide.
                UnOp::Plus => self.eval_ctx(*operand, w, eff_signed),
                UnOp::Minus => {
                    let a = self.eval_ctx(*operand, w, eff_signed);
                    self.negate(&a)                 // width-preserving, stays `w`
                }
                UnOp::BitNot => {
                    let a = self.eval_ctx(*operand, w, eff_signed);
                    let mut r = Value::zeros(a.width, eff_signed);
                    for k in 0..a.width { let (v,u)=not1(a.get_vu(k)); r.set_vu(k,v,u); }
                    r
                }
                // reductions + lognot: SELF-DETERMINED operand, 1-bit result,
                // then zero-extend to `w` (= self_width(1).max(ctx_width), always
                // unsigned). Using `w` (not `ctx_width.max(1)`) keeps the
                // extension target identical to the leaf arms above — no latent
                // off-by-context-width trap if a reduction self-width ever >1.
                UnOp::LogNot | UnOp::RedAnd | UnOp::RedNand | UnOp::RedOr
                | UnOp::RedNor | UnOp::RedXor | UnOp::RedXnor => {
                    let bit = self.eval_unary_self(*op, *operand); // 1-bit
                    bit.resize_keep_sign(w, false)                 // zero-extend
                }
            },

            // ── binary ─────────────────────────────────────────────────────
            Expr::Binary { op, lhs, rhs } => self.eval_binary_ctx(*op, *lhs, *rhs, w, eff_signed),

            // ── ternary: cond self-determined; branches context-determined ──
            Expr::Ternary { cond, then_e, else_e } => {
                match self.truthiness(&self.eval(*cond)) {   // cond at its OWN width
                    Tri::True  => self.eval_ctx(*then_e, w, eff_signed),
                    Tri::False => self.eval_ctx(*else_e, w, eff_signed),
                    Tri::Unknown => {
                        // both branches at (w, eff_signed); merge differing→X.
                        let t = self.eval_ctx(*then_e, w, eff_signed);
                        let e = self.eval_ctx(*else_e, w, eff_signed);
                        self.merge_x(&t, &e, w, eff_signed)
                    }
                }
            }

            // ── SELF-DETERMINED structural / select: eval natural, resize ──
            Expr::Concat { parts } => {
                let nat = self.eval_concat(parts);            // sum of self-widths
                nat.resize_keep_sign(w, false)                // concat unsigned
            }
            Expr::Replicate { count, value } => {
                let nat = self.eval_replicate(*count, *value);
                nat.resize_keep_sign(w, false)                // replicate unsigned
            }
            Expr::Select { base, offset, width, kind } => {
                let nat = self.eval_select(*base, *offset, *width, *kind); // unsigned
                nat.resize_keep_sign(w, false)                // select unsigned
            }

            // ── system functions ───────────────────────────────────────────
            // (eval_sysfunc_ctx already resizes to (w, eff_signed) internally.)
            Expr::SysFunc { which, args } => self.eval_sysfunc_ctx(*which, args, w, eff_signed),

            // ── user call: 1-bit X (v1), extend to `w` (= 1.max(ctx_width)) ──
            // elaborate v1 NEVER emits `Expr::Call` (inline_function always
            // returns an existing/lowered ExprId or a `Const` placeholder —
            // lib.rs:1636), so this arm is defensive/unreachable in practice,
            // matching eval.rs:53. Kept only for exhaustive-match safety.
            Expr::Call { .. } => Value::x1().resize_keep_sign(w, false),
        }
    }
```

### 4.4 Binary dispatch with correct context routing

```rust
    // `and1`/`or1`/`xor1`/`xnor1` are already imported at the top of eval.rs
    // (`use crate::value::{and1, ..};`); `BitOp` is the eval.rs-local type alias
    // (`type BitOp = fn((u64,u64),(u64,u64)) -> (u64,u64);`). No new imports.
    // `w` is the already-resolved eval width (= self_width.max(ctx_width)) passed
    // from eval_ctx; the comparison/logical arms zero-extend their 1-bit result to
    // `w` (= 1.max(ctx_width)), so no separate `ctx_width` parameter is needed.
    fn eval_binary_ctx(&self, op: BinOp, lhs: u32, rhs: u32, w: u32, eff_signed: bool) -> Value {
        use BinOp::*;
        match op {
            // ARITHMETIC — context-determined: BOTH operands sized to
            // (w, eff_signed), op at width w. eff_signed already AND-folds the
            // self-signedness with context (§5.5.1 global-unsigned rule).
            Add | Sub | Mul | Div | Mod => {
                let l = self.eval_ctx(lhs, w, eff_signed);
                let r = self.eval_ctx(rhs, w, eff_signed);
                self.arith(op, &l, &r)            // operates at max(l.w,r.w)=w
            }

            // POWER — base is context-determined; EXPONENT is SELF-DETERMINED
            // (IEEE: like a shift amount). `**` is signed iff the BASE is signed,
            // INDEPENDENT of the exponent. We therefore must NOT fold the
            // exponent's sign into the base's context: evaluate the base with the
            // BASE's own self-signedness ANDed with context (so an unsigned outer
            // context can still demote per §5.5.1, but an unsigned exponent can
            // NOT), and evaluate the exponent at its OWN self-width/sign.
            // `arith` self-determines its result width from its operands; the base
            // is already `w`-wide and dominates the (self-width) exponent, so the
            // result is `w`-wide with `both_signed = base_signed && exp_self_signed`
            // — but per IEEE the result sign must follow the BASE alone. We restamp
            // the exponent's effective sign to match the base so `arith`'s
            // both-signed reduction yields base-signedness without zero-extending
            // a signed base. (Numerically the exponent value is unchanged; only its
            // .signed attribute is aligned to avoid mis-signing the base context.)
            Pow => {
                // The Pow node's self-sign is the BASE sign alone (§3.3), so the
                // incoming `eff_signed` (= base.self_signed AND ctx_signed) is
                // already the base's effective sign — the exponent never entered
                // it. Evaluate the base in context with `eff_signed`; an unsigned
                // outer context can still demote (§5.5.1), an unsigned exponent
                // cannot. The exponent is self-determined (own width).
                let base = self.eval_ctx(lhs, w, eff_signed);
                // exponent self-determined (own width); restamp its sign to the
                // base's so arith's `both_signed` follows the base, not the exp.
                let mut exp = self.eval(rhs);
                exp.signed = base.signed;
                self.arith(op, &base, &exp)       // result width = base width = w
            }

            // BITWISE — context-determined: BOTH operands sized to (w, eff_signed).
            // `bitwise(l, r, f)` selects the per-bit primitive `f` by op.
            BitAnd | BitOr | BitXor | BitXnor => {
                let l = self.eval_ctx(lhs, w, eff_signed);
                let r = self.eval_ctx(rhs, w, eff_signed);
                let f: BitOp = match op {
                    BitAnd  => and1,
                    BitOr   => or1,
                    BitXor  => xor1,
                    BitXnor => xnor1,
                    _ => unreachable!("bitwise arm only handles BitAnd/Or/Xor/Xnor"),
                };
                self.bitwise(&l, &r, f)           // signature: (l, r, f)
            }

            // COMPARISONS / CASE-EQ — self-determined result (1-bit), but the two
            // operands are MUTUALLY context-determined: size each to
            // max(self_width(L), self_width(R)) with their pair-signedness. The
            // comparison does NOT inherit the enclosing `ctx_signed`/`eff_signed`
            // (it recomputes operand context from operand self-widths/sign) — this
            // correctly stops upward width/sign propagation.
            Lt | Le | Gt | Ge | Eq | Ne | CaseEq | CaseNe => {
                let cmp_w = self.wt.width(lhs).max(self.wt.width(rhs));
                let pair_signed = self.wt.signed(lhs) && self.wt.signed(rhs);
                let l = self.eval_ctx(lhs, cmp_w, pair_signed);
                let r = self.eval_ctx(rhs, cmp_w, pair_signed);
                let bit = match op {
                    CaseEq | CaseNe => self.case_eq(op, &l, &r),
                    Eq | Ne         => self.log_eq(op, &l, &r),
                    _               => self.relational(op, &l, &r),
                };
                bit.resize_keep_sign(w, false)  // zero-extend 1→w (= max(1,ctx))
            }

            // LOGICAL — self-determined operands, each reduced independently.
            LogAnd | LogOr => {
                let l = self.eval(lhs);   // OWN self-width
                let r = self.eval(rhs);
                let bit = if matches!(op, LogAnd) { self.log_and(&l, &r) }
                          else { self.log_or(&l, &r) };
                bit.resize_keep_sign(w, false)  // = max(1, ctx)
            }

            // SHIFTS — LEFT operand is context-determined (result width = LEFT/ctx
            // width = w); RIGHT operand (amount) is SELF-DETERMINED (own width).
            Shl | AShl => {
                let l = self.eval_ctx(lhs, w, eff_signed);   // widen LEFT FIRST
                let r = self.eval(rhs);                       // amount, own width
                let shifted = self.shift_left(&l, &r);        // grows then we clamp
                shifted.resize_keep_sign(w, eff_signed)       // back to ctx width
            }
            Shr => {
                let l = self.eval_ctx(lhs, w, eff_signed);
                let r = self.eval(rhs);
                self.shift_right(&l, &r, false)               // logical, fill 0
            }
            // ARITHMETIC RIGHT SHIFT — IEEE §5.5.1/§11.4.10: the sign-fill is
            // governed by the LEFT operand's OWN self-signedness, NOT the
            // enclosing context. An unsigned enclosing context MUST NOT demote a
            // genuinely-signed `s >>> n` to a logical (zero-fill) shift. We
            // therefore (a) evaluate the LEFT operand with its OWN self-sign so its
            // MSB carries the true sign bit, and (b) pass that same own-sign as the
            // `arith` fill flag to `shift_right`. Only AFTER shifting do we resize
            // to the surrounding (w, eff_signed) context.
            //   Worked example: `s` signed 8-bit = 0x80 (-128); `(s >>> 2)` used in
            //   an UNSIGNED 8-bit context. lhs_signed = wt.signed(s) = true (NOT
            //   AND-folded with ctx). l evaluated signed → MSB=1; fill=true →
            //   0xE0 (-32, arithmetic). The old `l.signed` (== eff_signed == false
            //   here) would have produced 0x20 (logical) — the bug.
            AShr => {
                let lhs_signed = self.wt.signed(lhs);         // OWN self-sign
                // widen LEFT to ctx width but keep its OWN sign for the fill MSB
                let l = self.eval_ctx(lhs, w, lhs_signed);
                let r = self.eval(rhs);
                let shifted = self.shift_right(&l, &r, lhs_signed); // arith iff LEFT signed
                shifted.resize_keep_sign(w, eff_signed)       // re-stamp to ctx sign
            }
        }
    }
```

**Notes on reused helpers (unchanged):**
- `arith`, `bitwise`, `relational`, `log_eq`, `case_eq`, `log_and`, `log_or`, `shift_left`, `shift_right`, `negate`, `reduce`, `reduce_not` keep their exact current bodies. We only change *who calls them and at what operand width*. (`merge_x` is NOT one of these — it is a NEW method holding the verbatim merge body lifted out of the now-deleted `eval_ternary` unknown branch; see §4.6.)
- `arith` self-determines `w = l.width.max(r.width)` — after `eval_ctx` both operands are already `w`-wide, so `arith` sees `max(w,w)=w` and its existing `both_signed = l.signed && r.signed` equals `eff_signed` (both operands stamped `eff_signed` by `resize_keep_sign`). **The carry now lands** because `w` already includes the LHS context width.
- `shift_left` grows `l.width + amt` (cap 4096) then we `resize_keep_sign(w, …)` back — identical truncation to today's `write_lvalue` path, just earlier and at the correct context width. For a left-shift into a WIDER lhs, widening LEFT first (to `w`) preserves the high bits IEEE requires.

### 4.5 Small extracted helpers

```rust
    /// 1-bit reduction/lognot result for a self-determined operand.
    fn eval_unary_self(&self, op: UnOp, operand: u32) -> Value {
        let a = self.eval(operand);              // OWN self width
        match op {
            UnOp::LogNot => match self.truthiness(&a) {
                Tri::True => Value::zeros(1,false), Tri::False => Value::one1(), Tri::Unknown => Value::x1(),
            },
            UnOp::RedAnd  => self.reduce(&a, and1),
            UnOp::RedNand => self.reduce_not(&a, and1),
            UnOp::RedOr   => self.reduce(&a, or1),
            UnOp::RedNor  => self.reduce_not(&a, or1),
            UnOp::RedXor  => self.reduce(&a, xor1),
            UnOp::RedXnor => self.reduce_not(&a, xor1),
            _ => unreachable!("eval_unary_self only for reductions/lognot"),
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
                // so `$signed(x)` must ZERO-extend there, not sign-extend.
                // `resize_keep_sign(w, eff_signed)` already AND-folds self.signed
                // with eff_signed and stamps the result .signed = eff_signed — so
                // setting `a.signed = eff_signed` BEFORE the resize makes the fill
                // policy unambiguously driven by the explicit flag (no reliance on a
                // stale `signed=true` field that a later self-determined re-read
                // could mis-extend). For eff_signed=true this sign-extends; for
                // eff_signed=false it zero-extends.
                let mut a = self.eval(args[0]);
                a.signed = eff_signed;
                a.resize_keep_sign(w, eff_signed)
            }
            SysFuncId::Unsigned => {
                let mut a = self.eval(args[0]); a.signed = false;
                a.resize_keep_sign(w, false)        // unsigned cast → zero-extend
            }
            // $time/$realtime (64-bit) and $clog2 (32-bit): natural value, then
            // resize to context (zero/sign per eff_signed; both are conventionally
            // unsigned/integer so eff_signed only matters if used in signed ctx).
            _ => self.eval_sysfunc(which, args).resize_keep_sign(w, eff_signed),
        }
    }
```

### 4.6 The ternary-unknown merge, extracted verbatim

The existing `eval_ternary` unknown-branch body (`eval.rs:394–407`) becomes a method so `eval_ctx`'s Ternary arm can call it after sizing both branches to `(w, eff_signed)`:

```rust
    fn merge_x(&self, t: &Value, e: &Value, w: u32, signed: bool) -> Value {
        let mut out = Value::zeros(w, signed);
        for k in 0..w {
            let a = t.get_vu(k); let b = e.get_vu(k);
            if a == b { out.set_vu(k, a.0, a.1); } else { out.set_vu(k, 0, 1); }
        }
        out
    }
```

(Both `t` and `e` are already `w`-wide from `eval_ctx`, so no inner `.resize` is needed — the only behavioral delta vs. today is that `w` now includes context width and both branches were extended with `eff_signed`, matching IEEE both-signed.)

---

## 5. INTEGRATION — every RHS-against-lvalue call site

The rule at each site: compute `lhs_width = Σ chunk_width(c)` over `lhs.chunks`, then evaluate the RHS as

```
eval_ctx(rhs, max(lhs_width, self_width[rhs].0), self_width[rhs].1)
```

**The LHS SIGN does NOT propagate** — only its width. The RHS's internal sign is `self_width[rhs].1` (the rhs's own self-signedness). `write_lvalue` afterwards still truncates/extends to the exact lhs width (no corruption — §5.5).

### 5.1 Helper on `SimState` (lhs width, no sign)

`chunk_width` is already a method on `SimState` (`state.rs:129`). Add a public sibling:

```rust
// state.rs (impl SimState)
/// Total destination bit-width of an lvalue (Σ chunk widths). Used to seed the
/// RHS context width. Does NOT compute a sign — lhs sign never propagates.
pub(crate) fn lvalue_width(&self, lhs: &Lvalue) -> u32 {
    lhs.chunks.iter().map(|c| self.chunk_width(c)).sum::<u32>().max(1)
}
```

The scheduler exposes a façade that pairs it with the width table:

```rust
// sched.rs (impl Scheduler) — NEW
/// Evaluate `rhs` in the context of `lhs`'s width (IEEE assignment rule):
/// width = max(lhs_width, self_width(rhs)); sign = rhs self-sign (lhs sign
/// does NOT propagate).
pub(crate) fn eval_for_lvalue(&self, lhs: &Lvalue, rhs: u32) -> Value {
    let lw = self.st.lvalue_width(lhs);
    let sw = self.st.wt.get(rhs);
    let ctx_w = lw.max(sw.width);
    self.eval_ctx_top(rhs, ctx_w, sw.signed)
}

/// Build an EvalCtx and run eval_ctx (mirror of the `eval` façade, sched.rs:366).
pub(crate) fn eval_ctx_top(&self, eid: u32, ctx_width: u32, ctx_signed: bool) -> Value {
    let ctx = EvalCtx { ir: self.st.ir, nets: self.st, now: self.st.now, wt: &self.st.wt };
    ctx.eval_ctx(eid, ctx_width, ctx_signed)
}
```

The plain `eval`/`truthy` façades (`sched.rs:366`, `sched.rs:375`) just gain the `wt: &self.st.wt` field in the `EvalCtx { … }` literal — no behavior change (they still self-size).

### 5.2 Site 1 — continuous assign (`sched.rs:101–119`, `settle_cont_assigns`)

**Before:**
```rust
for ci in 0..self.st.ir.cont_assigns.len() {
    let ca_rhs = self.st.ir.cont_assigns[ci].rhs;
    let v = self.eval(ca_rhs);
    let lhs = self.st.ir.cont_assigns[ci].lhs.clone();
    changed |= self.st.write_lvalue(&lhs, v);
}
```
**After:**
```rust
for ci in 0..self.st.ir.cont_assigns.len() {
    let ca_rhs = self.st.ir.cont_assigns[ci].rhs;
    let lhs = self.st.ir.cont_assigns[ci].lhs.clone();
    let v = self.eval_for_lvalue(&lhs, ca_rhs);   // CONTEXT-SIZED to lhs width
    changed |= self.st.write_lvalue(&lhs, v);
}
```

### 5.3 Site 2 — blocking assign (`exec.rs:38–41`)

**Before:**
```rust
Stmt::BlockingAssign { lhs, rhs } => {
    let v = sched.eval(rhs);
    sched.st.write_lvalue(&lhs, v);
}
```
**After:**
```rust
Stmt::BlockingAssign { lhs, rhs } => {
    let v = sched.eval_for_lvalue(&lhs, rhs);     // CONTEXT-SIZED to lhs width
    sched.st.write_lvalue(&lhs, v);
}
```

### 5.4 Site 3 — nonblocking assign / NBA (`exec.rs:42–45`)

**Before:**
```rust
Stmt::NonblockingAssign { lhs, rhs } => {
    let sampled = sched.eval(rhs); // sample RHS now (Active)
    sched.schedule_nba(lhs, sampled);
}
```
**After:**
```rust
Stmt::NonblockingAssign { lhs, rhs } => {
    let sampled = sched.eval_for_lvalue(&lhs, rhs); // CONTEXT-SIZED, sampled now
    sched.schedule_nba(lhs, sampled);
}
```
The sampled value is already `max(lhs_width, self_width)`-wide; `schedule_nba` stores it verbatim and the NBA-region `write_lvalue` (via `schedule_nba` → applied later) truncates/extends to lhs width identically to a blocking write. **No change to `schedule_nba` or the NBA application path.**

### 5.5 Site 4 — port connect

**None.** Research confirms there is **no port/instance-connection eval path** in the engine — elaborate flattens instances into the net arena before sim (`grep` for port/connect/first_net in sim-engine returns nothing). **No edit required.** A future port-connect path, when added, will route through `eval_for_lvalue` against the receiving net's width (noted for forward-compat; out of scope here).

### 5.6 SysTask args (`builtins.rs:193/239/312`) — UNCHANGED

`$display`/`$write` args are **self-determined** (rendered to a string, no lvalue). They keep `sched.eval(eid)` (self-width). No context to propagate. Listed here only to confirm we deliberately do **not** touch them.

---

## 5.5 `write_lvalue` post-extension is still correct (no double-extension corruption)

`write_lvalue` (`state.rs:106–125`) does `let src = value.resize(total.max(1));` where `total = Σ chunk_width`. After our change `value.width == max(lhs_width, self_width) ≥ lhs_width = total`:

- If `value.width == total`: `resize` hits the `new_width == self.width` fast path (`value.rs:190`) → `mask_top` only, **byte-identical** to a value that was already the right width.
- If `value.width > total` (self_width exceeded lhs, e.g. concat wider than lhs): `resize` **truncates** to `total` — the high bits are dropped exactly as IEEE assignment requires (the store side truncates).
- The extension policy in this final `resize` follows `value.signed`, which is `eff_signed` from the top context. Since `value.width ≥ total`, the *extension* branch (`new_width > self.width`) is **never taken** here — only truncation or no-op. So the sign used for the store-side fill is irrelevant when `value.width ≥ total`; **no double sign-extension can occur.** The only place a value is *extended* into a wider lhs is inside `eval_ctx` (controlled, with `eff_signed` already AND-folded), never twice.

**Confirmed:** the existing `write_lvalue` truncate/extend stays, and double-extension cannot corrupt because the value handed to it is already ≥ lhs width.

---

## 6. WHERE THE TABLE LIVES + THREADING

### 6.1 Home: `SimState`

`SimState` owns `ir: &'a SimIr` and is borrowed (`nets: &self`) by every `EvalCtx`. It is constructed once (`lib.rs:116`) before any eval. Add the table there:

```rust
// state.rs
pub(crate) struct SimState<'a> {
    pub ir: &'a SimIr,
    pub now: u64,
    pub nets: Vec<NetSlot>,
    pub wt: crate::width::WidthTable,   // NEW — built once, immutable for the run
    // ... vcd, out, flags unchanged ...
}
```

**Build site — `SimState::new` (`state.rs:50–91`):** add one line where `nets` is already built:

```rust
let nets = ir.nets.iter().map(/* unchanged */).collect();
let wt = crate::width::WidthTable::build(ir);   // NEW: single forward pass
SimState { ir, now: 0, nets, wt, /* ...rest unchanged... */ }
```

`build` is O(#exprs), runs once at init, allocates one `Vec<SelfWidth>` (8 bytes/entry). No per-eval allocation.

### 6.2 `EvalCtx` construction sites gain `wt: &self.st.wt`

There are **two** `EvalCtx { … }` literals today, both in `sched.rs`:
- `eval` façade (`sched.rs:366–372`)
- `truthy` façade (`sched.rs:375–381`)

Both become:
```rust
let ctx = EvalCtx { ir: self.st.ir, nets: self.st, now: self.st.now, wt: &self.st.wt };
```
plus the new `eval_ctx_top` (§5.1) uses the same literal. **No other construction sites exist** (verified: `grep "EvalCtx {" crates/sim-engine` → only `sched.rs`).

### 6.3 Borrow check

`self.st.wt` and `self.st` (as `&dyn NetReader` via `nets: self.st`) are both **shared** borrows of `*self.st` — compatible. `EvalCtx` holds `ir: &SimIr`, `nets: &SimState`, `wt: &WidthTable`, all `&'a` of the same `SimState`. `eval_ctx` is `&self`, no mutation. Clean.

---

## 7. UNIT TESTS (18, exact assertions)

File: `crates/sim-engine/src/width.rs` `#[cfg(test)] mod tests` for table values + a `tests/width_inference.rs` integration test for end-to-end sizing through a tiny hand-built `SimIr`. Tests build minimal `SimIr`s directly (no parser dependency) using the frozen constructors, then assert on `eval_ctx` / `eval_for_lvalue` results.

Helper (test-local): `mk_net(width, signed)`, `mk_const(width, signed, val)`, `bin(op, l, r)`, and an `eval_top(ir, eid, ctx_w, ctx_signed)` building a no-VCD `SimState`.

### T1 — self-width table values (table sanity)
4-bit unsigned net `a`, 5-bit signed net `b`, `a+b`, `a==b`, `{a,b}`, `a<<3`.
```
assert_eq!(wt.get(a),   SelfWidth{width:4, signed:false});
assert_eq!(wt.get(b),   SelfWidth{width:5, signed:true});
assert_eq!(wt.get(add), SelfWidth{width:5, signed:false}); // max(4,5), not both signed
assert_eq!(wt.get(eq),  SelfWidth{width:1, signed:false});
assert_eq!(wt.get(cat), SelfWidth{width:9, signed:false}); // 4+5, unsigned
assert_eq!(wt.get(shl), SelfWidth{width:4, signed:false}); // LEFT width, amount irrelevant
```

### T2 — Worked example 1: narrow-add carry into wide lhs
`a=4'd9, b=4'd9` (unsigned), `sum` 5-bit. `eval_for_lvalue(sum_lhs, a+b)`.
```
let v = sched.eval_for_lvalue(&sum_lhs, add);   // ctx_width = max(5,4) = 5
assert_eq!(v.width, 5);
assert_eq!(v.to_u64(), Some(18));               // 9+9=18 — carry in bit 4 PRESERVED
```
Regression contrast: today `eval(add)` gives width 4, `to_u64()==2` (18 & 0xF).

### T3 — Worked example 2: 1-bit comparison zero-extended into wide AND
`x=y` (8-bit equal), `mask=8'hF0`, `result = (x==y) & mask`, result 8-bit.
```
// (x==y)→1 (1-bit), AND ctx widens it to 8 → 0x01, & 0xF0 = 0x00
let v = sched.eval_for_lvalue(&result_lhs, and_expr);
assert_eq!(v.width, 8);
assert_eq!(v.to_u64(), Some(0x00));
// x != y case: (x==y)→0, & 0xF0 = 0x00 too; use mask=8'hFF + equal → 0x01:
```
Second assert with `mask=8'h01`, `x==y` true: `0x01 & 0x01 = 0x01`. The comparison never sign-replicates: `0x01`, not `0xFF`.

### T4 — Worked example 3: signed × unsigned becomes unsigned
`s` signed 8-bit = `-1` (`0xFF`), `u` unsigned 8-bit = `2`, `p = s*u`, `p` 16-bit.
```
let v = sched.eval_for_lvalue(&p_lhs, mul);     // ctx 16; u unsigned ⇒ whole mul unsigned
assert_eq!(v.width, 16);
assert_eq!(v.to_u64(), Some(0x00FF * 2));        // s ZERO-extended to 0x00FF, ×2 = 0x01FE = 510
assert_eq!(v.signed, false);
```
Contrast (wrong/naive): sign-extending `s`→`0xFFFF` (=-1) gives `-2 = 0xFFFE`.

### T5 — Worked example 4a: arithmetic shift sign (`>>>` on signed)
`s` signed 8-bit = `0x80` (-128), `r = s >>> 2`, `r` 8-bit signed.
```
let v = sched.eval_for_lvalue(&r_lhs, ashr);
assert_eq!(v.width, 8);
assert_eq!(v.to_u64(), Some(0xE0));              // -128>>>2 = -32 = 0xE0 (sign-filled)
```

### T6 — Worked example 4b: logical shift on same value (`>>`)
`r = s >> 2` (logical).
```
assert_eq!(v.to_u64(), Some(0x20));              // zero-filled: 0x80>>2 = 0x20
```

### T7 — Worked example 5: concat operand NOT widened by context
`a`,`b` 4-bit; `w = {a,b}`, `w` 12-bit. `a=4'hA`, `b=4'h5`.
```
let v = sched.eval_for_lvalue(&w_lhs, cat);      // cat self-width 8, ctx 12
assert_eq!(v.width, 12);
assert_eq!(v.to_u64(), Some(0x0A5));             // {A,5}=0xA5 then zero-extended to 12 bits
```
Crucially `a` is NOT pre-widened to 12 (that would misplace `b`); inner concat stays 8 then extends.

### T8 — Worked example 6: ternary branch widening
`hi` 8-bit=`0xAB`, `lo` 4-bit=`0xC`, `c=1'b0`, `out = c ? hi : lo`, `out` 8-bit.
```
let v = sched.eval_for_lvalue(&out_lhs, tern);   // ctx 8; lo widened to 8
assert_eq!(v.width, 8);
assert_eq!(v.to_u64(), Some(0x0C));              // lo selected, delivered 8-bit (0x0C), upper zeros
```
With `c=1'b1`: `Some(0xAB)`.

### T9 — comparison operands mutually sized (narrow vs wide)
`n` 4-bit=`0xF` (15), `m` 8-bit=`0x0F` (15), `n==m`.
```
let v = sched.eval(eq);                          // self-determined 1-bit
assert_eq!(v.width, 1);
assert_eq!(v.to_u64(), Some(1));                 // 15==15: n zero-extended to 8 before compare
```
And `m=0xFF`: `Some(0)` (15 != 255). Verifies cross-sizing without leaking width up.

### T10 — shift amount is self-determined (does not pull context)
`l` 4-bit=`0x3`, amount `sh` 32-bit=`1`, `out=l<<sh`, `out` 8-bit.
```
let v = sched.eval_for_lvalue(&out_lhs, shl);    // LEFT widened to 8, amount stays own width
assert_eq!(v.width, 8);
assert_eq!(v.to_u64(), Some(0x06));              // 3<<1=6, no bits lost (left widened first)
```
With `out` 4-bit: `Some(0x06)` still (6 fits); with `l=0x9, out 4-bit`: `9<<1=18 & 0xF = 2`.

### T11 — `$signed`/`$unsigned` width+sign
`u` unsigned 4-bit=`0xF`, `e = $signed(u) + 4'sd0` evaluated into 8-bit signed lhs.
```
// $signed(u): width 4 preserved, signed=true. In a both-signed +, sign-extends 0xF→0xFF...
let v = sched.eval_for_lvalue(&s8_lhs, add_signed);
assert_eq!(v.to_u64(), Some(0xFF));              // -1 sign-extended to 8 bits
// $unsigned of a signed −1 4-bit:
let w = sched.eval_for_lvalue(&u8_lhs, unsigned_of_neg);
assert_eq!(w.to_u64(), Some(0x0F));              // zero-extended, stays 15
```

### T12 — `$clog2` fixed 32-bit, `$time` 64-bit
```
assert_eq!(wt.get(clog2_expr), SelfWidth{width:32, signed:true});
assert_eq!(wt.get(time_expr),  SelfWidth{width:64, signed:false});
let v = sched.eval(clog2_expr_of_const_8);
assert_eq!(v.width, 32); assert_eq!(v.to_u64(), Some(3)); // clog2(8)=3
```

### T13 — REGRESSION: equal-width expr — new `eval` == new `eval_for_lvalue`
8-bit `a`,`b`, `c = a & b`, `c` 8-bit (widths already match). Build the value via BOTH the (rewritten) self-determined `eval(and)` and new `eval_for_lvalue(c_lhs, and)`.
```
let old = sched.eval(and);                        // self-width = 8 = ctx
let new = sched.eval_for_lvalue(&c_lhs, and);     // ctx_width = max(8,8) = 8
assert_eq!(old.width, new.width);
assert_eq!(old.val, new.val);
assert_eq!(old.unk, new.unk);
assert_eq!(old.signed, new.signed);               // equal-width self vs ctx: identical
```
Property-style extension: a small fixed corpus of equal-width exprs (add/and/or/xor/concat-into-exact/ternary-equal-branches) all assert `eval == eval_for_lvalue` field-by-field.

> **SCOPE CAVEAT (read before relying on T13).** Because the public `eval` is rewritten in §4.2 to delegate to `eval_ctx`, BOTH sides of T13 route through the NEW engine — so T13 proves new-`eval`-self ≡ new-`eval_for_lvalue` for the equal-width case (the resize fast-paths to `mask_top`, a no-op), NOT byte-identity against the PRE-change engine. **The true pre/post regression gate is the existing `crates/sim-engine/tests/end_to_end.rs` suite** (display/arith/nba/vcd goldens), whose expected outputs were captured against the old engine and are NOT regenerated by this change (only the NEW carry-smoke golden in §8 is added). **The implementer MUST run the FULL `cargo test --workspace --locked` suite** — a green `end_to_end.rs` is what certifies no behavioral regression for self-determined top-level use; T13 only certifies the self↔ctx equivalence at equal width.

### T14 — X/Z preservation unchanged under widening
`a` 4-bit = `4'b10x1`, `sum = a + 4'd0` into 8-bit lhs. The X bit is preserved and the extension is clean.
```
let v = sched.eval_for_lvalue(&w8_lhs, add0);
assert_eq!(v.width, 8);
assert_eq!(v.get_vu(1), (0,1));                   // bit1 still X
assert_eq!(v.get_vu(4), (0,0));                   // zero-extended (unsigned)
```

### T15 — MANDATORY: arithmetic `>>>` keeps sign-fill under an UNSIGNED context
Pins BLOCKER fix (§4.4 AShr): an unsigned enclosing context must NOT demote a signed `>>>` to a logical shift. `s` signed 8-bit = `0x80` (-128); `r = (s >>> 2)` written into an **UNSIGNED** 8-bit lvalue `u8`.
```
let v = sched.eval_for_lvalue(&u8_lhs, ashr);    // ctx unsigned (eff_signed=false)
assert_eq!(v.width, 8);
assert_eq!(v.to_u64(), Some(0xE0));              // ARITHMETIC fill (sign-bit), -32
// REGRESSION GUARD: the buggy `l.signed`(==eff_signed==false) path would give 0x20.
assert_ne!(v.to_u64(), Some(0x20));
```
Contrast: `$unsigned(s) >>> 2` (left operand made unsigned) → logical fill `0x20`, pinned by an analogous case if `$unsigned` is available in the test corpus.

### T16 — MANDATORY: `**` keeps BASE sign under an UNSIGNED exponent
Pins MAJOR fix (§3.3 Pow split + §4.4 Pow arm): an unsigned exponent must NOT demote a signed base. `b` signed 4-bit = `0xF` (-1), exponent `e` unsigned = `3`; `p = b ** e` into a signed 8-bit lvalue.
```
assert_eq!(wt.get(pow), SelfWidth{width:4, signed:true});  // max(4, e.width)→base width; BASE-signed
let v = sched.eval_for_lvalue(&s8_lhs, pow);
assert_eq!(v.to_u64(), Some(0xFF));              // (-1)**3 = -1 = 0xFF (sign-extended)
// REGRESSION GUARD: the buggy both-signed fold would zero-extend base→+15, 15**3=3375.
assert_ne!(v.to_u64(), Some(3375 & 0xFF));
```

### T17 — MANDATORY: `$signed(x)` in an UNSIGNED region zero-extends
Pins MAJOR fix (§4.5 `$signed`): the global-unsigned rule makes `$signed`'s fill follow `eff_signed`, not the unconditional cast. `u4` unsigned 4-bit = `0xF`; `r = $signed(u4) | s8` where the OR has an unsigned sibling so the region is unsigned (`eff_signed=false`); `s8 = 8'h00`.
```
let v = sched.eval_for_lvalue(&u8_lhs, or_expr); // region unsigned
assert_eq!(v.to_u64(), Some(0x0F));              // $signed(0xF) ZERO-extended to 0x0F, |0x00
// REGRESSION GUARD: a fragile unconditional sign-extend would give 0xFF.
assert_ne!(v.to_u64(), Some(0xFF));
```

### T18 — MANDATORY: const-folded part-select width feeds parent context
Pins MAJOR fix (§3.4 `Add(Sub(msb,lsb),1)` fold): a `[msb:lsb]` part-select's self-width must be the range width, not the fallback 1, so a part-select used as a context-determined operand is not under-sized. `c` 12-bit; `sel = c[11:4]` (8-bit range); `out = c[11:4] + d[3:0]` into a wide lvalue.
```
assert_eq!(wt.get(sel), SelfWidth{width:8, signed:false}); // (11-4)+1 = 8, NOT fallback 1
let v = sched.eval_for_lvalue(&out_lhs, add);    // add sized to max(8,4)=8, not 4
assert_eq!(v.width.max(out_w), out_w);           // select contributed its full 8 bits
```
REGRESSION GUARD: with the old `Sub`-only fold, `wt.get(sel).width == 1`, the add sizes to `max(1,4)=4`, truncating the select — the exact bug this spec exists to fix, reproduced for part-selects.

---

## 8. CLI-LEVEL SMOKE (carry-preservation observable via `vita`)

A design where width inference **changes the simulated VCD/output**, runnable end-to-end:

```verilog
module carry_smoke;
  reg  [3:0] a, b;
  reg  [4:0] sum;        // 5-bit: must capture the carry-out
  initial begin
    a = 4'd9; b = 4'd9;
    sum = a + b;         // 18 — needs bit 4
    $display("sum=%0d", sum);
    #1 $finish;
  end
endmodule
```

**Run:** `vita carry_smoke.v` (or `vrun` after `velab`).
**Before this change:** `a+b` evaluates at 4-bit, `to_u64()==2`, then writes to `sum` ⇒ `$display` prints `sum=2` and `sum` VCD shows `00010`.
**After:** RHS context-sized to `max(5,4)=5`, `a`/`b` zero-extended to 5 bits, add at 5 bits ⇒ `18`, `$display` prints `sum=18`, VCD `sum` = `10010`.

The corpus runner (`crates/corpus-runner`) gains this as a golden fixture: a `.v` plus expected RTL output `sum=18` and a VCD asserting `sum=10010` at t=0. This is the user-visible proof the carry is preserved. (The determinism golden gate is unaffected — the IR/`schema_hash` root is untouched; only engine numeric output changes, which is exactly the intended fix and gets a NEW golden, not a regression of the frozen root.)

---

## 9. `$signed` / `$unsigned` and fixed-width SysFuncs (summary, §3.3/§4.5/§6)

- **`$signed(x)`**: self-width = **operand self-width** (preserved), self-signed = **true** in the side table. In eval, the operand is read at its own width and the **extension fill follows `eff_signed`** (= the `$signed` node's self-sign AND the context sign), NOT an unconditional sign-extend: in a both-signed region it sign-extends (T11), but under the global-unsigned rule (an unsigned sibling, §5.5.1) it ZERO-extends (T17). The value's `.signed` field is set to `eff_signed` before the resize so the fill policy is unambiguously flag-driven (§4.5). Matches `eval_sysfunc` (`eval.rs:486–490`) except now context-aware.
- **`$unsigned(x)`**: self-width = operand width, self-signed = **false**; eval zero-extends to context. Matches `eval.rs:491–495`.
- **`$clog2(x)`**: fixed **32-bit signed** `integer` return. Width never depends on the argument.
- **`$time` / `$realtime`**: fixed **64-bit unsigned** (`$realtime` modeled as 64-bit integral in v1 — real type out of scope, §10).
- All four pre-existed in `eval_sysfunc`; the width pass simply pins their self-width so a SysFunc embedded in a context-determined expression sizes correctly (e.g. `x + $clog2(y)` now sizes the add to `max(width(x), 32)`).

---

## 10. OUT OF SCOPE (explicit)

1. **`real` / `realtime` floating types** — `$realtime` is modeled as 64-bit integral (existing `eval_sysfunc` behavior); true IEEE real arithmetic and real-↔integral coercion are out of scope.
2. **Non-constant part-select width** — `Select.width` that does not const-fold (a runtime-variable range) falls back to width 1 in the table; `eval_select` already clamps at runtime. v1 elaborate only emits constant part widths, so this is unreachable in practice.
3. **User function return-width** (`Expr::Call`) — v1 eval returns 1-bit X (`eval.rs:53`); the table mirrors that (width 1). Real `FuncDef` return-net resolution is v2.
4. **4-state corner cases already handled** — X/Z poisoning in arithmetic, Z→X normalization, the wide-signed `arith` poison guard (`eval.rs:182–184`), out-of-range select→X: all **unchanged**. Width inference only re-sizes operands *before* these helpers run; their X/Z semantics are untouched (verified by T14).
5. **Self-determined sizing of `$display` args** — rendered to string, not assigned; no context. Unchanged.
6. **Pow (`**`) value semantics / division-by-zero** — IN SCOPE for *sizing*: Pow width = `max(L,R)` (Table 5-22) but its self-sign and exponent handling are explicitly corrected (§3.3 Pow arm: self-sign = BASE sign only; §4.4 Pow arm: exponent SELF-DETERMINED, an unsigned exponent never demotes a signed base; pinned by T16). OUT of scope: the numeric `**`/div-by-zero VALUE semantics inside `arith`/`ipow_signed`, which are unchanged.
7. **Port-connect width coercion** — no such path exists in the engine (§5.5); forward-compat note only.

## 11. DIAG CODES — NONE NEEDED (bijection gate untouched)

This change adds **no new diagnostics**. Width inference is a silent, correctness-improving re-sizing of operands; it never reports an error or warning. The existing `MsgCode` set (36, doc-15 bijection) is **unchanged**, so the bijection gate (`diag` ↔ `docs/preview/15-error-code-reference.md`) and `schema_hash` golden root remain green. Malformed/degenerate cases (non-const select width, oversize replication) **clamp** silently (matching the engine's existing v1 "RunRange simplification" / out-of-range→X philosophy at `state.rs:177`), they do not raise diagnostics. **If** a future decision wants a `WidthTruncation` lint, the promotion would be: add ONE `MsgCode::ElabWidthTrunc` (Warning), add its row to doc-15, regenerate the bijection fixture — but that is **explicitly not part of this spec**.

---

## 12. FILE-CHANGE MANIFEST (for the implementer)

| File | Change |
|---|---|
| `crates/sim-engine/src/width.rs` | **NEW** — `SelfWidth`, `WidthTable`, `build`, `self_width_of`, `const_u32_of_expr`, `clamp/add/mul_w`, `#[cfg(test)]` table tests (T1, T12 table parts). |
| `crates/sim-engine/src/lib.rs` (module root) | add `mod width;`. |
| `crates/sim-engine/src/state.rs` | `SimState` gains `wt: WidthTable`; `SimState::new` builds it; add `lvalue_width`. |
| `crates/sim-engine/src/eval.rs` | `EvalCtx` gains `wt` borrow; `eval` delegates to `eval_ctx`; add `eval_ctx`, `eval_binary_ctx`, `eval_unary_self`, `eval_sysfunc_ctx`, `merge_x`. **DELETE `eval_unary` (`:73`), `eval_binary` (`:135`), `eval_ternary` (`:386`)** — zero callers after the rewrite, denied by the `dead_code` lint gate (§4.2). Leaf helpers (`arith`/`bitwise`/`relational`/`log_eq`/`case_eq`/`log_and`/`log_or`/`reduce`/`reduce_not`/`negate`/`shift_left`/`shift_right` + `and1`/`or1`/`xor1`/`xnor1`/`not1`) all retain callers via the new methods → unchanged, kept. |
| `crates/sim-engine/src/sched.rs` | 2 `EvalCtx{…}` literals gain `wt`; add `eval_for_lvalue`, `eval_ctx_top`; `settle_cont_assigns` uses `eval_for_lvalue`. |
| `crates/sim-engine/src/exec.rs` | blocking + NBA sites use `sched.eval_for_lvalue(&lhs, rhs)`. |
| `tests/width_inference.rs` (sim-engine) | T2–T11, T13–T18 end-to-end (T15 AShr-in-unsigned-ctx, T16 Pow base-sign, T17 `$signed`-in-unsigned-region, T18 part-select-width-fold are MANDATORY regression pins for the resolved BLOCKER/MAJOR defects). |
| `crates/corpus-runner` fixture | `carry_smoke.v` golden (§8). |

**Frozen, NOT touched:** all of `sim-ir`, `vita-schema`, `vita-artifact`, `diag` MsgCode set, `docs/preview/15`/`16`/`17`. `schema_hash::<SimIr>()` root unchanged.
