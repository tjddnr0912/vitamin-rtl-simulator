//! S2 (R2) — width-specialized expression programs for the tier-3 walk.
//!
//! The profile that motivated this (keccak_f_flat, release, `/usr/bin/sample`):
//! ~60% of the native walk was `Value` manipulation (`mask_top` 16%, `resize`
//! 12%, `has_xz`/`from_packed`/`to_u64` ~12%) plus the generic recursive
//! evaluator (`eval_ctx` 16%, `eval_binary_ctx` 12%) and `read_net` boxing
//! (6.5%). None of that is the arithmetic itself — it is the 72-byte value
//! representation, exactly what doc-21 §2 said R2 must remove.
//!
//! ## What this module admits, and why admission is the correctness argument
//!
//! A `WProg` is compiled for an expression tree only when EVERY node is:
//!
//! - `Const` (Numeric) whose self width and sign equal the context's,
//! - `Signal` reading a whole ≤64-bit integral net, a CONSTANT in-bounds element
//!   of a ≤64-bit-element array (bounds and 2-state-ness checked at compile
//!   time), or — since S2 slice 4 — a RUNTIME element whose index expression is
//!   itself admitted,
//! - `BitNot` / `BitAnd` / `BitOr` / `BitXor` / `Add` / `Sub`,
//! - `Shl` / `Shr` / `AShr` by a 2-state constant amount — except a SIGNED
//!   `AShr`, whose sign fill is the one shift whose bits depend on the sign,
//! - `Lt` / `Le` / `Gt` / `Ge` / `Eq` / `Ne` / `CaseEq` / `CaseNe`, whose two
//!   operands are MUTUALLY context-determined at `max(self-width)` with their
//!   pair signedness (§11.8.1 — the comparison node's own context is one
//!   unsigned bit, and it does NOT inherit the enclosing one),
//! - `LogAnd` / `LogOr`, whose operands are SELF-determined and need not share
//!   a width with each other or with the result,
//! - `LogNot` and the six reductions, over a self-determined operand.
//! - `Concat` and `Replicate`, whose parts are self-determined and TILE the
//!   result exactly (S2 slice 5),
//! - `Select` over a self-determined base at a CONSTANT, provably in-range
//!   offset (S2 slice 6) — one shift and one mask,
//! - `Ternary` whose branches CANNOT REPORT (S2 slice 7) — the one admission in
//!   this module that is about evaluation order rather than width.
//!
//! A node carries its context's width and signedness unless it is
//! SELF-determined, in which case it carries its own and is converted on the way
//! out. Six positions introduce a FURTHER width without any conversion — a
//! comparison's operands, a runtime array index, the operands of `&&`/`||` and of
//! the one-bit unaries, a concat/replicate part, and a select's base — and one,
//! the widening admission in `compile_node`, is a genuine CONVERSION.
//! Four widths in ONE program is an
//! ordinary shape, not a corner: `(sa < sb) && m[idx]` gives 1 / 8 / 8 / 4.
//! The first four DISCARD their operand — every one of those nodes yields a
//! single bit and `LoadIdx` discards the index entirely — so the further width
//! dies where it is introduced. The fifth does not: a concat part's bits SURVIVE
//! into a wider result, and the sentence that used to stand here ("no value is
//! ever moved between widths") stopped being true when slice 5 admitted it.
//! What used to stand here in its place — *"no widening, no sign extension and
//! no truncation exists anywhere in an admitted tree"* — has ALSO expired, and
//! deliberately: the widening admission in `compile_node` is exactly a widening
//! and, for a signed value in a signed context, exactly a sign extension.
//!
//! ⚠️ So the correctness argument is no longer "there is no conversion". It is
//! **every conversion is the generic path's own**: the only one this module emits
//! is `WOp::Sext`, which calls `value::resize_word` — the single spelling
//! `Value::resize`'s ≤64-bit arm uses — and it is emitted under exactly
//! `resize_keep_sign`'s condition (`value.signed && ctx_signed`). TRUNCATION still
//! exists nowhere: it declines. And WHICH nodes may be converted is decided by
//! `node_ctx_class` against the LRM's sizing rules, because converting a
//! CONTEXT-determined operator instead of computing it at the context width is a
//! wrong answer (`v[8:11] + 4'd1` is 16 at eight bits and 0 at four).
//!
//! The rest of that paragraph still stands as written. A part contributes EXACTLY its own bits at a
//! compile-time offset, the offsets tile the result (`Σ pw == w`, checked), and
//! the accumulator they merge into is definite zero — so there is no room for a
//! fill and no bit is written twice. A select is the mirror image and the same
//! claim holds for the same reason: it takes a proven-in-range window of its
//! base's bits and discards the rest, so nothing is extended and nothing is
//! silently cut (a cut the SOURCE did not authorise is what `bits != w` and the
//! range test refuse). The context-sizing rules the generic
//! evaluator implements therefore still have nothing to do, and this module does
//! not restate them (the classifier-must-match-lowering trap). Signedness is admitted
//! because at uniform width two's complement makes every op above produce the
//! same BITS either way; that is measured by the battery, not assumed, and the
//! two places it is NOT true are handled explicitly (the signed `AShr`
//! declines, and a comparison hands its operand sign to the shared function).
//!
//! What remains is the per-op 4-state BIT SEMANTICS, and those are pinned by an
//! exhaustive per-bit-state differential against the generic evaluator plus the
//! corpus mirror sweep (`s2_wprog_*` tests) — measured equal, not argued equal.
//! Nothing here is restated: the comparisons call the shared
//! `eval::binops::{relational, log_eq, case_eq}`, `&&`/`||` call
//! `eval::binops::log_bin_tri`, and `!` plus the six reductions call
//! `eval::unary_self_of` — the same function the generic `eval_unary_self`
//! reaches. Those are free functions extracted for this; the pre-existing
//! `EvalCtx` methods delegate, so the generic path is unchanged.
//!
//! ## Which EVALUATIONS reach this module — narrower than "the walk"
//!
//! THREE call sites compile programs: `k_eval_for_lvalue` (the rhs of a blocking
//! assign, an NBA sample, a `force`, and a cont-assign settle), `k_truthy`
//! (branch conditions and the `wait(e)` predicate), and — since S2 slice 3 —
//! `ensure_index_kind`, which resolves an lvalue's own index expressions so
//! `fast_offsets` can answer without the generic evaluator. (That third one was
//! `index_of` until S2 slice 4 split deciding from running; both `index_of` and
//! `index_admits` reach it.) Everything else
//! still evaluates generically: a comparison inside a SYSTEM TASK argument
//! (`$display("%b", a < b)`) goes through `eval_task_arg`, and any lvalue the
//! fast path declines goes through `eval::resolve_offsets` whole.
//!
//! The count is stated because the previous version of this paragraph said
//! "two entry points … and the lvalue's own offsets go through
//! `resolve_offsets`", which slice 3 made false in the same commit that added
//! the third caller. A coverage claim that does not name its entry points is
//! not a coverage claim — and one that names the wrong number understates the
//! blast radius.
//!
//! ## Representation
//!
//! `W = (val, unk)` — one net bit per plane bit, `unk=1` ⇒ x (`val=0`) or z
//! (`val=1`), the arena's own plane encoding, so `Load` is two u64 reads at
//! compile-time-resolved buffer indices (the slot invariant keeps bits above
//! `width` zero, so loads need no masking). Every stack value stays masked to
//! ITS OWN width: loads are pre-masked, `Const` masks at compile time, and
//! each op that can move bits out of range (`Not`, `And`, `Or`, `Shl`, `Add`,
//! `Sub`) carries that operand's mask. The mask rides the OP rather than the
//! program because a program can hold two widths at once — a comparison's
//! operands are `ow` bits wide while its result is one.
//!
//! ## The admission's SIGN half — measured and REJECTED (2026-08-20)
//!
//! `compile_node`'s entry gate is `sw.width != w || sw.signed != signed`. An
//! execution-weighted census of picorv32 (200k cycles) found that gate is where
//! **90.4% of the generic-path evaluations decline**, split 47.6% for sign alone
//! and 42.8% for a narrower inner node. The root's sign always matches by
//! construction — `k_eval_for_lvalue` passes the RHS's own — so every sign
//! decline is an INNER node inheriting its parent's context sign.
//!
//! Dropping the sign half is SOUND, and the argument is the admitted set rather
//! than a claim about two's complement: `Div`/`Mod`/`Mul`/`Pow` are not admitted,
//! `Add`/`Sub` produce identical bits either way, the bitwise ops and `Shl` are
//! sign-blind, `Shr` is logical for both signs, and the two ops that DO read a
//! sign take it from the operand (`>>>` would have to ask `wt.get(lhs).signed`
//! instead of the context; a comparison already passes `lw.signed`).
//!
//! It was built, and it does widen admission: the slow lane fell 600,045 →
//! 485,757 evaluations, **-19.0%**. And it is **1.00x** — picorv32 2.39 s → 2.39 s,
//! keccak 0.47 s → 0.47 s, best-of-5, noise floor ~1%. Reverted rather than
//! shipped, the same call `levelize.rs` records for the rank-ordered drain.
//!
//! ⚠️ The gap between 47.6% and 19.0% is the lesson, not a mistake in either
//! number: a decline-site histogram attributes the FIRST failure, and removing
//! that gate only helps a tree that fails nowhere else. Most of these trees hit
//! another gate immediately after. Expect first-failure attribution to overstate
//! a fix by roughly this factor.
//!
//! The arithmetic for why -19% of the slow lane is invisible: slow evaluations
//! are ~6x a fast one and the whole lvalue-eval subtree is ~10% of the run, so
//! the move is worth ~0.8% — under the noise floor before it is written. That
//! division is the thing to do BEFORE building the next one of these.

use sim_ir::SimIr;

use crate::native::arena::NetArena;
use crate::value::Value;
use crate::width::WidthTable;

/// One ≤64-bit 4-state value: the arena's two planes, one word each.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct W {
    pub val: u64,
    pub unk: u64,
}

#[derive(Clone, Copy, Debug)]
enum WOp {
    /// Push a constant (pre-masked at compile time).
    Const {
        val: u64,
        unk: u64,
    },
    /// Push the net value at buffer indices `(vi, vi+1)` — val plane word,
    /// then unk plane word, resolved from the slot descriptor at compile time.
    Load {
        vi: u32,
    },
    /// S2 slice 4: push the ELEMENT the top of stack names. Pops the index
    /// (already computed at ITS own self-width by the ops just emitted) and
    /// pushes `[val, unk]` at `off + idx*2`, or all-X when the index is not a
    /// clean in-range element — which also COUNTS a deferred out-of-range
    /// report, because that is the arm `read_net` reports from.
    ///
    /// `words == 1` is an admission precondition, so the stride is 2.
    LoadIdx {
        off: u32,
        elems: u32,
        /// The ELEMENT width's mask — what an out-of-range read fills with x.
        m: u64,
    },
    // Every op that can move bits out of its operand's range carries THAT
    // operand's mask, not the program's: since S2 slice 2 a program can hold
    // two widths (a comparison's operands are `ow` bits wide, its result is 1).
    Not {
        m: u64,
    },
    And {
        m: u64,
    },
    Or {
        m: u64,
    },
    Xor,
    Shl {
        k: u32,
        m: u64,
    },
    Shr {
        k: u32,
    },
    Add {
        m: u64,
    },
    Sub {
        m: u64,
    },
    /// One-bit-result LOGICAL binary (`&&`, `||`). Unlike a comparison, the two
    /// operands are SELF-determined and need not share a width, so each carries
    /// its own — the value on the stack was masked to it when it was compiled.
    LogBin {
        op: sim_ir::BinOp,
        lw: u32,
        ls: bool,
        rw: u32,
        rs: bool,
    },
    /// One-bit-result unary over a SELF-determined operand: `!` or one of the six
    /// reductions. The mapping is not restated here — `unary_self_of` is the same
    /// function the generic evaluator reaches through `eval_unary_self`.
    Unary1 {
        op: sim_ir::UnOp,
        ow: u32,
        os: bool,
    },
    /// The ordered / equality comparisons. Pops two operand-width values,
    /// pushes a 1-bit result computed by the SHARED `eval::binops` free
    /// functions — the 4-state rules there (an ambiguous compare is x, but a
    /// definite mismatch decides `==` even with x elsewhere) are exactly the
    /// kind this module must never restate.
    Cmp {
        op: sim_ir::BinOp,
        ow: u32,
        osigned: bool,
    },
    /// S2 slice 7: `?:`. Pops the ELSE value, the THEN value and the COND
    /// (pushed in that order) and pushes the branch the condition selects, or
    /// the bitwise X-merge when the condition is unknown — the same three
    /// outcomes `eval_ctx`'s `Ternary` arm has, through the same two shared
    /// functions (`truthiness`, `merge_x_word`).
    ///
    /// ⚠️ Both branches have ALREADY been evaluated when this runs. That is the
    /// one place this module departs from the generic path's SHAPE, and the
    /// compiler pays for it with an admission check rather than an argument —
    /// see the compile arm.
    Tern {
        cw: u32,
        cs: bool,
        m: u64,
    },
    /// SIGN-EXTEND the top of stack from `from` bits to `to`.
    ///
    /// Only ever emitted for a SIGNED value in a SIGNED context. Zero-extension
    /// needs no op at all: the module's standing invariant is that every stack
    /// value is masked to its own width ("loads are pre-masked, `Const` masks at
    /// compile time, and each op that can move bits out of range carries that
    /// operand's mask" — the `LogBin` arm already rests on it), so widening a
    /// value whose high bits are zero is the identity on the word pair.
    ///
    /// ⚠️ The extension itself is NOT restated here — it calls `value::resize_word`,
    /// which is the one spelling the generic path uses through
    /// `Value::resize`'s ≤64-bit arm. An x or z in the sign bit fills the new bits
    /// with x/z, which is exactly the rule that would be easy to get wrong twice.
    Sext {
        from: u32,
        to: u32,
    },
    /// S2 slice 6: the top of stack's bits `[k +: w]`, i.e. one shift and one
    /// mask on both planes. `copy_bits(out, 0, src, k, w)` into a zeroed `w`-bit
    /// destination is exactly this, and the compiler only emits it after proving
    /// the range lies wholly inside the source — so the generic path's X-filling
    /// arms have no counterpart here because they are unreachable.
    Slice {
        k: u32,
        m: u64,
    },
    /// S2 slice 5: merge the popped value into the accumulator BENEATH it —
    /// `count` copies of its `stride`-spaced bits starting at bit `off`, masked
    /// to the result width. One op serves both a concatenation part
    /// (`count: 1`) and a replication (`count: n, stride: operand width`).
    ///
    /// The merge is a plane-wise OR rather than a 4-state one because the
    /// destination range is definite ZERO on entry — the accumulator is a
    /// `Const{0,0}` and the ranges tile without overlap. That is the identical
    /// contract `eval::eval_core::copy_bits` states for its own destination.
    Splice {
        off: u32,
        stride: u32,
        count: u32,
        m: u64,
    },
}

/// The two evaluation stacks, held together so a caller takes ONE borrow.
///
/// `u` is the 2-state lane's (D2): eight bytes per entry instead of sixteen,
/// which is the larger half of what that lane buys on the shapes the census
/// found to be pure data movement (`struct-heavy` is 45% `Splice` + 28% `Load`
/// + 17% `Const`).
#[derive(Default)]
pub(crate) struct WScratch {
    pub(crate) w: Vec<W>,
    pub(crate) u: Vec<u64>,
}

/// A program that is a SINGLE leaf, recognised once at compile time so `run`
/// can answer it without entering an executor.
///
/// ⭐ Measured before building: of the programs actually executed, **56.4% on
/// serv and 60.0% on picorv32 are one `Load` or one `Const`**. Those paid a
/// scratch `clear` + `reserve`, a loop set-up, one match dispatch, a push and a
/// pop — to read two adjacent words. Nothing about that is compilation; there
/// is no code to generate, only an interpreter not to enter.
///
/// The collapse is exact on BOTH lanes, which is why it is a `match` on the
/// program rather than a heuristic:
///
/// * `Load { vi }` — `run_4s` pushes `W { buf[vi], buf[vi+1] }`. `run_2s`
///   declines when `buf[vi+1] != 0` and otherwise yields `W { buf[vi], 0 }`,
///   which is the same value BECAUSE it only takes that arm when the unk word
///   is zero.
/// * `Const { val, unk }` — `run_4s` pushes `W { val, unk }`; `two_state` is
///   false whenever any `Const` carries `unk != 0`, so the 2-state lane is only
///   reached when `unk == 0` and yields `W { val, 0 }`.
///
/// Neither arm masks, and neither did the ops: an arena slot is already
/// canonical at its own width (the admission gate requires `slot.width == w`),
/// which is the invariant the `Load` arm has always relied on.
#[derive(Clone, Copy)]
enum WFast {
    /// Run the ops.
    Prog,
    Leaf {
        vi: u32,
    },
    Lit {
        val: u64,
        unk: u64,
    },
}

pub(crate) struct WProg {
    ops: Vec<WOp>,
    /// The RESULT's width and signedness — what the caller stamps on the
    /// `Value` it builds. Operand widths live in the ops that need them.
    width: u32,
    signed: bool,
    /// Maximum stack depth, so the executor can reserve once.
    depth: usize,
    /// Single-leaf short circuit — see [`WFast`].
    fast: WFast,
    /// D2: may this program ever attempt the 2-state lane?
    ///
    /// FALSE when a `Const` carries x/z, because that leaf is unknown on every
    /// evaluation and the lane would bail on every run — the flag turns a
    /// per-run bail into a compile-time one. Nothing else is decided here: the
    /// other two leaves (`Load`, `LoadIdx`) are runtime facts and the lane
    /// checks them itself.
    two_state: bool,
}

/// Compile `eid` for context `(w, signed)` — `None` when any node falls
/// outside the admitted set. Pure function of the IR: programs never need
/// invalidation.
/// Can a context of width `w` reach this evaluator AT ALL?
///
/// `compile`'s first line, named so the admitted width has one spelling.
///
/// ⚠️⚠️ **IT IS NECESSARY AND NOT SUFFICIENT, and D1.5→D1.6 paid for learning
/// that.** D1.5 used exactly this as the boundary that decides whether tier-3
/// hands an RHS to `native_eval` — attractive because it answers without walking
/// the expression. But `compile` also declines on NODE KINDS (a runtime-offset
/// part-select, a `SysFunc`, an operand mix it has no arm for), and a census
/// counted **75 RHSs at ≤64 bits** that this predicate waves through and
/// `compile` then refuses. Each went to neither evaluator, and `struct-heavy`
/// stayed 1.30× slower than the VM until the boundary asked `compile` itself
/// (127 → 86 ms · ROADMAP §5.1-ax).
///
/// So: use this to explain the width rule, never to predict an admission. The
/// only honest answer to "will `wprog` take this?" is to run `compile`.
pub(crate) fn width_admits(w: u32) -> bool {
    w != 0 && w <= 64
}

pub(crate) fn compile(
    ir: &SimIr,
    wt: &WidthTable,
    arena: &NetArena,
    eid: u32,
    w: u32,
    signed: bool,
) -> Option<WProg> {
    if !width_admits(w) {
        return None;
    }
    let mut ops = Vec::new();
    let mut depth = 0usize;
    let mut max_depth = 0usize;
    compile_node(
        ir,
        wt,
        arena,
        eid,
        w,
        signed,
        &mut ops,
        &mut depth,
        &mut max_depth,
    )?;
    let two_state = !ops
        .iter()
        .any(|o| matches!(*o, WOp::Const { unk, .. } if unk != 0));
    let fast = match ops.as_slice() {
        [WOp::Load { vi }] => WFast::Leaf { vi: *vi },
        [WOp::Const { val, unk }] => WFast::Lit {
            val: *val,
            unk: *unk,
        },
        _ => WFast::Prog,
    };
    Some(WProg {
        ops,
        width: w,
        signed,
        depth: max_depth,
        two_state,
        fast,
    })
}

#[inline]
fn mask_of(w: u32) -> u64 {
    if w >= 64 {
        u64::MAX
    } else {
        (1u64 << w) - 1
    }
}

#[allow(clippy::too_many_arguments)]
fn compile_node(
    ir: &SimIr,
    wt: &WidthTable,
    arena: &NetArena,
    eid: u32,
    w: u32,
    signed: bool,
    ops: &mut Vec<WOp>,
    depth: &mut usize,
    max_depth: &mut usize,
) -> Option<()> {
    // UNIFORM SIGN inside one node's subtree, and uniform WIDTH except where the
    // branch below converts. Signedness was excluded outright until S2 slice 2; at
    // uniform width it is inert for every op admitted below (two's complement makes
    // the BITS identical), which the exhaustive battery measures rather than
    // assumes — and the one place a width is now crossed emits the generic path's
    // OWN conversion rather than relying on that inertness.
    // The two places sign is NOT inert are handled explicitly: an arithmetic
    // right shift declines when signed, and a comparison passes the operand
    // sign to the shared comparison functions.
    let sw = wt.get(eid);
    // ── A NARROWER node in a WIDER context ────────────────────────────────
    //
    // ⚠️⚠️ `sw.width < w` does NOT mean "compute at `sw.width` and extend". That is
    // true only for a SELF-DETERMINED node. A CONTEXT-DETERMINED operator is
    // performed at the CONTEXT width (§11.6.1), and folding it at its own width
    // first is a wrong answer, not a slower one: `logic [7:0] s = v[8:11] + 4'd1`
    // with the select all-ones is 15 + 1 = **16** at the assignment's 8 bits and
    // **0** at the addition's own 4. That is a pinned test, and it is what the
    // first version of this branch broke.
    //
    // So the two halves are handled differently, and which half a node is in is
    // decided by [`node_ctx_class`] rather than by its width:
    //
    // * SELF-determined (a leaf, a select, a concat/replication, and every
    //   one-bit result — comparisons, `&&`/`||`, `!` and the reductions): the
    //   value is fixed at its own width, so compile it there and EXTEND.
    // * CONTEXT-determined (`~`, the bitwise binaries, `+`/`-`, a shift's left
    //   operand, a ternary's branches): the operator is performed at `w`, so
    //   simply proceed — the arms already hand `w` to their own
    //   context-determined children, and a narrower LEAF underneath lands in the
    //   self-determined half above.
    //
    // The sign rule is `resize_keep_sign`'s, not a new one: it sets
    // `signed = self.signed && ctx_signed` BEFORE resizing, so the fill is a sign
    // fill only when the value AND its context are both signed, and zero otherwise.
    // Zero-extension needs no op at all — see [`WOp::Sext`].
    //
    // ⚠️ TRUNCATION (`sw.width > w`) still declines everywhere. The generic path
    // truncates there, and nothing measured asks for it: every decline the corpus
    // census attributed to this gate was a widening.
    //
    // ⚠️ The census that motivates this is in `docs/study/03-workload-corpus.md`:
    // on darkriscv it is 84% of all declined requests (comparison operands of
    // unequal width, which reach this through the arm below), and the same shape
    // is the top bucket on serv and picorv32.
    if sw.width != w {
        match node_ctx_class(ir, eid) {
            CtxClass::SelfDetermined => {
                if sw.width == 0 || sw.width > w || !width_admits(sw.width) {
                    return None;
                }
                compile_node(
                    ir, wt, arena, eid, sw.width, sw.signed, ops, depth, max_depth,
                )?;
                if sw.signed && signed {
                    ops.push(WOp::Sext {
                        from: sw.width,
                        to: w,
                    });
                }
                return Some(());
            }
            // Performed at `w`; fall through to the arms, which mask to `w`.
            CtxClass::ContextDetermined => {}
            // Anything this module has no arm for declines here exactly as the
            // blanket width gate used to — the catch-all is the status quo, so a
            // new `Expr` variant cannot be silently mis-sized by this branch.
            CtxClass::Unknown => return None,
        }
    }
    // ⚠️⚠️ The SIGN half of this gate applies to every node EXCEPT a `Const` leaf.
    //
    // The module header above records that dropping the sign half entirely was built,
    // measured SOUND, measured **1.00x** on picorv32 and keccak, and reverted. That
    // measurement was right and its conclusion was scoped to those two designs. An
    // external round-34 report brought a shape neither of them has, and there the same
    // gate costs 2.9x:
    //
    //   64 continuous assigns `assign n_i = s ^ II;`, 200k cycles, release, interleaved
    //     localparam logic [31:0] II   (unsigned)   native 41.1 ns/eval   vm 72.8   0.56x
    //     localparam int          II   (SIGNED)     native 118.1 ns/eval  vm 73.2   1.61x
    //
    // A `localparam int` is what SV RTL writes, and a genvar in an expression is the
    // same cell — so a `generate for` whose body indexes with its genvar, which is what
    // a hash or cipher round looks like, falls off this path entirely.
    //
    // Restricting the relaxation to `Const` keeps the soundness argument the header
    // already makes, and makes it trivial rather than set-wide: at EQUAL WIDTH a
    // constant's two's-complement BITS do not depend on how its signedness was
    // recorded, and the `Const` arm below masks to `w` and pushes exactly those bits.
    // The two ops that read a sign are unaffected — `>>>` declines on the NODE's sign,
    // and a comparison requires both operands to share a signedness and passes it down
    // explicitly, so a mixed-sign comparison still declines at its own guard.
    //
    // ⭐⭐ THE EXEMPTION COVERS `Signal` TOO. The argument is not "the same BITS either
    // way" — it is that the arm never asks: NO exit of the `Signal` arm reads `signed` or
    // `sw.signed`, directly or through a mask. It requires `slot.width == w`, and the
    // arena's slot invariant (established at the WRITE producers, `arena.rs` and
    // `write.rs`, which all mask) is what lets the load skip a mask of its own.
    //
    // ⚠️ BOTH sub-paths of that arm are admitted, not just the scalar one. A constant
    // index gives `Load { vi }`, two word reads at a compile-time offset; a RUNTIME index
    // compiles the index at its OWN `(iw.width, iw.signed)` and emits `LoadIdx`. The
    // second is the one that also moves a DIAGNOSTIC — `LoadIdx` is this module's only
    // caller of `note_bad_index` — and it is genuinely newly reached: a signed-element
    // memory read through a runtime index in an unsigned context measured **2.3x**.
    // (Naming only the scalar half is how a coverage claim turns into a smaller one than
    // the code makes; the review caught exactly that here.)
    //
    // The other half of the obligation is that the generic path this must equal changes
    // no plane bit at equal width — `resize_keep_sign`'s `w == self.width` arm writes
    // only `signed`. That is the same rule the widening admission above cites.
    //
    // ⚠️ This is the narrow half of the relaxation §5.1 measured and reverted. That
    // measurement — dropping the sign half for EVERY node, 1.00x — was over picorv32 and
    // keccak, and its own note says the conclusion was scoped to those two designs. A
    // fresh execution-weighted census puts **6,600,872 requests from exactly TWO
    // expressions** on this gate in `darkriscv` (92% of that design's declines), which
    // was not in the original pair. Leaves only, so nothing about the reverted set-wide
    // relaxation is reinstated.
    if sw.signed != signed
        && !matches!(
            ir.exprs.get(eid as usize)?,
            sim_ir::Expr::Const { .. } | sim_ir::Expr::Signal { .. }
        )
    {
        return None;
    }
    let mask = mask_of(w);
    let push = |ops: &mut Vec<WOp>, depth: &mut usize, max_depth: &mut usize, op: WOp| {
        ops.push(op);
        *depth += 1;
        *max_depth = (*max_depth).max(*depth);
    };
    match ir.exprs.get(eid as usize)? {
        sim_ir::Expr::Const { val } => {
            let (cv, cu) = const_planes(ir, *val, w)?;
            push(
                ops,
                depth,
                max_depth,
                WOp::Const {
                    val: cv & mask,
                    unk: cu & mask,
                },
            );
            Some(())
        }
        sim_ir::Expr::Signal { net, word } => {
            // §2 row 33: the same read-through the interpreter applies, resolved
            // at compile time from the same table.
            let net = &wt.read_alias(eid).unwrap_or(*net);
            // KIND first, and locally rather than by inheritance. Today the
            // arena refuses `NetKind::Real` at build ("real: S2 width class"),
            // so a real net cannot reach here — but this arm's checks are all
            // about SHAPE (`width`/`words`/`elems`), and a real slot has the
            // shape of a 64-bit integer while its bits are an f64. Whoever
            // lifts that arena row would otherwise silently hand `-0.0` to the
            // integer path, where `truthiness` reads a set sign bit as TRUE —
            // the exact trap that function's own doc names. The guard is one
            // `matches!`; the precondition it replaces was three files away.
            let kind = ir.nets.get(*net as usize)?.kind;
            if !matches!(
                kind,
                sim_ir::NetKind::Wire
                    | sim_ir::NetKind::Reg
                    | sim_ir::NetKind::Logic
                    | sim_ir::NetKind::Integer
            ) {
                return None;
            }
            // A3-ii-a: DECLINE a frame slot. This arm resolves the net to an
            // arena slot at COMPILE time, and a frame-local net's slot is dead —
            // its value is in the activation window. The generic evaluator below
            // reads through the composite `NetReader`, which routes; the
            // specialised one cannot, so the only correct answer here is to fall
            // through. (Measured: without this, every formal read `x` while the
            // module net beside it was right.)
            if arena.frame.get(*net as usize).copied().unwrap_or(false) {
                return None;
            }
            // A2-i: DECLINE a class handle, and unconditionally rather than only
            // for a field select. A field read (`word = Some(field_id)`) has to
            // go to `class_heap`, which this compiled form cannot reach; a BARE
            // handle read is in the slot and would be answered correctly — but
            // the `word` here is an INDEX EXPRESSION, and telling a field id
            // from an array index by inspecting it is a second spelling of the
            // routing rule. Declining the net is the same answer with one
            // spelling. (Measured cost: a design that only copies handles loses
            // the fast path, which is not a correctness surface.)
            if arena.class.get(*net as usize).copied().unwrap_or(false) {
                return None;
            }
            arena.assert_owns(*net, "wprog::compile Expr::Signal");
            let slot = arena.slots.get(*net as usize)?;
            if slot.width != w || slot.words != 1 {
                return None;
            }
            // `word` is the INDEX EXPRESSION's id, not an element number. A
            // 2-state Numeric constant in bounds folds at compile time; anything
            // else is admitted as a RUNTIME index since S2 slice 4, which is why
            // the E4002 machinery had to come with it (`LoadIdx` reports).
            let e = match word {
                None => {
                    if slot.elems != 1 {
                        return None;
                    }
                    0u64
                }
                Some(weid) => {
                    match ir.exprs.get(*weid as usize)? {
                        sim_ir::Expr::Const { val } => {
                            let (iv, iu) = const_planes(ir, *val, 64)?;
                            if iu != 0 {
                                return None;
                            }
                            if iv >= u64::from(slot.elems) {
                                return None;
                            }
                            iv
                        }
                        // RUNTIME INDEX (S2 slice 4). Compiled INLINE at its OWN
                        // self-determined width and sign — which is what the
                        // generic path evaluates it at (`self.eval(weid)`), and
                        // which a program is already allowed to hold alongside the
                        // value's width (slice 2 put the mask on the op, not the
                        // program). If the index will not compile, the whole tree
                        // declines exactly as before.
                        //
                        // The reason the constant arm above still exists is not
                        // symmetry: a constant in-bounds index proves at COMPILE
                        // time that no E4002 is reachable, so it emits a plain
                        // `Load` with no bounds test in the loop.
                        _ => {
                            // The width guard is NOT decoration: this is the one
                            // place a NEW context width enters the recursion, so it
                            // restates `compile`'s own entry precondition. Without
                            // it `mask_of(0)` is 0 and a >64-bit index would be
                            // truncated into the one-word `W` silently.
                            //
                            // ⚠️ An earlier version of this note claimed the seal
                            // normalises every array-word index to exactly 32 bits,
                            // so "no index of width != 32 arrives here". The round-2
                            // soundness review MEASURED that false — a 2-bit
                            // `Concat` arrives from the packed-element domain, and
                            // `packed.rs` has three more carve-outs that return an
                            // index unsealed. Those all decline at the `Concat`, so
                            // there is no consequence today; what is retracted is
                            // the reason, not the guard. Deleting this guard or
                            // `slot.words != 1` survives the suite, which is a
                            // statement about which widths reach here, not about
                            // whether they may.
                            let iw = wt.get(*weid);
                            if iw.width == 0 || iw.width > 64 {
                                return None;
                            }
                            compile_node(
                                ir, wt, arena, *weid, iw.width, iw.signed, ops, depth, max_depth,
                            )?;
                            // net 0: pops the index, pushes the element.
                            ops.push(WOp::LoadIdx {
                                off: slot.off,
                                elems: slot.elems,
                                m: mask,
                            });
                            return Some(());
                        }
                    }
                }
            };
            let vi = slot.off + (e as u32) * 2; // words == 1: [val, unk] adjacent
            push(ops, depth, max_depth, WOp::Load { vi });
            Some(())
        }
        sim_ir::Expr::Unary { op, operand } => {
            // `!` and the six reductions: SELF-determined operand, one unsigned
            // result bit. Same shape as the `&&`/`||` arm BELOW — the operand is
            // compiled at its own width so the shared mapping sees the bits it
            // expects, and the result discards it.
            if matches!(
                op,
                sim_ir::UnOp::LogNot
                    | sim_ir::UnOp::RedAnd
                    | sim_ir::UnOp::RedNand
                    | sim_ir::UnOp::RedOr
                    | sim_ir::UnOp::RedNor
                    | sim_ir::UnOp::RedXor
                    | sim_ir::UnOp::RedXnor
            ) {
                // Insurance, not a live check: `compile_node`'s own entry
                // precondition already requires the node's self width and sign to
                // equal the context's, and `sim_ir::selfwidth` gives every op in
                // this list `{width: 1, signed: false}` — so an unreachable
                // combination. Measured: replacing this `return` with a `panic!`
                // is never hit across the suite or the real designs. It stays
                // because the width the op RECORDS must be one bit whatever a
                // future width rule says, and that is cheaper to keep than to
                // re-derive.
                if w != 1 || signed {
                    return None; // the result IS one unsigned bit
                }
                let ow = wt.get(*operand);
                if ow.width == 0 || ow.width > 64 {
                    return None;
                }
                compile_node(
                    ir, wt, arena, *operand, ow.width, ow.signed, ops, depth, max_depth,
                )?;
                ops.push(WOp::Unary1 {
                    op: *op,
                    ow: ow.width,
                    os: ow.signed,
                });
                return Some(());
            }
            if !matches!(op, sim_ir::UnOp::BitNot) {
                return None;
            }
            compile_node(ir, wt, arena, *operand, w, signed, ops, depth, max_depth)?;
            ops.push(WOp::Not { m: mask });
            Some(())
        }
        sim_ir::Expr::Binary { op, lhs, rhs } => {
            use sim_ir::BinOp as B;
            match op {
                B::BitAnd | B::BitOr | B::BitXor | B::Add | B::Sub => {
                    compile_node(ir, wt, arena, *lhs, w, signed, ops, depth, max_depth)?;
                    compile_node(ir, wt, arena, *rhs, w, signed, ops, depth, max_depth)?;
                    *depth -= 1; // binary: two pops, one push
                    ops.push(match op {
                        B::BitAnd => WOp::And { m: mask },
                        B::BitOr => WOp::Or { m: mask },
                        B::BitXor => WOp::Xor,
                        B::Add => WOp::Add { m: mask },
                        _ => WOp::Sub { m: mask },
                    });
                    Some(())
                }
                // ORDERED / EQUALITY comparisons (S2 slice 2). The result is
                // ONE bit while the operands are `ow` bits — one of the two nodes
                // whose subtree width differs from its own (the other is a runtime
                // array index, S2 slice 4). The comparison itself is the shared
                // free function.
                //
                // ⚠️ This arm USED to demand that both operands already share a
                // width and a signedness, "so no §11.8.1 mixed-sign or widening
                // question arises here either". That is no longer the rule: the
                // question is asked and answered below, the same way
                // `eval_binary_ctx` answers it.
                B::Lt | B::Le | B::Gt | B::Ge | B::Eq | B::Ne | B::CaseEq | B::CaseNe => {
                    if w != 1 || signed {
                        return None; // a comparison's own self-width IS 1, unsigned
                    }
                    // §11.8.1: the two operands are MUTUALLY context-determined —
                    // each is sized to max(self-width) with their PAIR signedness.
                    // This is `eval_binary_ctx`'s comparison arm verbatim
                    // (`cmp_w = width(l).max(width(r))`, `pair = signed(l) &&
                    // signed(r)`), and the widening admission above is what lets the
                    // narrower side reach it — that is the whole reason this arm used
                    // to demand equal widths.
                    //
                    // ⚠️ Byte-identical for everything that compiled before: when the
                    // widths already matched, `ow` IS that width, and when the
                    // signednesses already matched, `os` IS that signedness.
                    let lw = wt.get(*lhs);
                    let rw = wt.get(*rhs);
                    let ow = lw.width.max(rw.width);
                    let os = lw.signed && rw.signed;
                    if ow == 0 || ow > 64 {
                        return None;
                    }
                    compile_node(ir, wt, arena, *lhs, ow, os, ops, depth, max_depth)?;
                    compile_node(ir, wt, arena, *rhs, ow, os, ops, depth, max_depth)?;
                    *depth -= 1;
                    ops.push(WOp::Cmp {
                        op: *op,
                        ow,
                        osigned: os,
                    });
                    Some(())
                }
                // `&&` / `||` — SELF-determined operands, each reduced to a
                // truth value independently, one unsigned result bit. Each operand
                // is compiled at ITS OWN width and sign, so the stack value is
                // already masked to that width and the truthiness scan sees exactly
                // the bits the generic one sees.
                //
                // The operands may differ in width from each other and from this
                // node — that is why the op carries both, and why this does not
                // violate the module's uniform-width admission: like a comparison,
                // it introduces a further width rather than a conversion, and the
                // result discards the operands entirely.
                //
                // ⚠️⚠️ This arm used to carry a SECOND justification — *"the generic
                // evaluator does NOT short-circuit … so evaluating both eagerly here
                // is not a behavioural choice; it is the same order"* — and that
                // sentence was load-bearing precisely because an admitted subtree can
                // still COUNT an out-of-range element read (`LoadIdx`). The generic
                // evaluator short-circuits now (IEEE 1800 §11.4.7), so the sentence
                // is false and the hazard it named is live: this lane always runs the
                // right operand, so a `LoadIdx` there would report an access the
                // generic path never performs.
                //
                // The answer is the one the `Ternary` arm below already gives, asked
                // the same way — of the compiled OPS rather than the expression shape,
                // which makes it exact rather than conservative. Only the RIGHT
                // operand is guarded: the left one is evaluated on every path, so a
                // report from it is not a divergence. A diagnostic APPEARING is a
                // divergence exactly as much as one going missing.
                B::LogAnd | B::LogOr => {
                    // Insurance, not a live check — see the identical note on the
                    // unary arm above.
                    if w != 1 || signed {
                        return None; // the result IS one unsigned bit
                    }
                    let lw = wt.get(*lhs);
                    let rw = wt.get(*rhs);
                    if lw.width == 0 || lw.width > 64 || rw.width == 0 || rw.width > 64 {
                        return None;
                    }
                    compile_node(
                        ir, wt, arena, *lhs, lw.width, lw.signed, ops, depth, max_depth,
                    )?;
                    let rhs_at = ops.len();
                    compile_node(
                        ir, wt, arena, *rhs, rw.width, rw.signed, ops, depth, max_depth,
                    )?;
                    if ops[rhs_at..]
                        .iter()
                        .any(|o| matches!(o, WOp::LoadIdx { .. }))
                    {
                        return None;
                    }
                    *depth -= 1;
                    ops.push(WOp::LogBin {
                        op: *op,
                        lw: lw.width,
                        ls: lw.signed,
                        rw: rw.width,
                        rs: rw.signed,
                    });
                    Some(())
                }
                B::Shl | B::Shr | B::AShr => {
                    // An ARITHMETIC right shift fills with the left operand's
                    // own sign bit; at uniform admission that sign is this
                    // node's, so a signed `>>>` is the one shift whose bits
                    // differ from the logical one. Decline it rather than
                    // restate the fill (`Shr` is logical for both signs, and
                    // `Shl` moves bits the same way either way).
                    if signed && matches!(op, B::AShr) {
                        return None;
                    }
                    // Amount: self-determined 2-state CONSTANT only.
                    let k = match ir.exprs.get(*rhs as usize)? {
                        sim_ir::Expr::Const { val } => {
                            let aw = wt.get(*rhs).width.min(64);
                            let (av, au) = const_planes(ir, *val, aw)?;
                            if au != 0 {
                                return None;
                            }
                            av
                        }
                        _ => return None,
                    };
                    // The LHS is compiled FIRST in every case — including
                    // `k >= w`. The first spelling short-circuited that case to
                    // `Const{0,0}` without visiting the lhs, which admitted
                    // trees whose lhs would have DECLINED: a dynamic OOB index
                    // lost its E4002 (loud → silent, exit class flipped) and a
                    // `$urandom` operand lost its draw (the whole subsequent
                    // RNG stream shifted — silent-wrong at exit 0). Both were
                    // measured by the soundness review; the admission IS the
                    // correctness argument, so no arm may skip it.
                    compile_node(ir, wt, arena, *lhs, w, signed, ops, depth, max_depth)?;
                    if k >= u64::from(w) {
                        // Every bit leaves. The compiled lhs is pure (SysFunc
                        // never admits), so its value is simply annihilated:
                        // AND with definite-0 is definite-0 for every 4-state
                        // input, x/z included — the same all-0 the generic
                        // path produces by actually shifting.
                        push(ops, depth, max_depth, WOp::Const { val: 0, unk: 0 });
                        *depth -= 1;
                        ops.push(WOp::And { m: mask });
                        return Some(());
                    }
                    ops.push(match op {
                        B::Shl => WOp::Shl {
                            k: k as u32,
                            m: mask,
                        },
                        _ => WOp::Shr { k: k as u32 },
                    });
                    Some(())
                }
                _ => None,
            }
        }
        // ── CONCATENATION / REPLICATION (S2 slice 5) ───────────────────────
        //
        // The first nodes this module admits whose operands live at a DIFFERENT
        // width from the result and whose bits SURVIVE into it. The four earlier
        // exceptions (comparison operands, a runtime array index, `&&`/`||` and
        // the one-bit unaries) all discard their operand, so the module doc could
        // say no value is ever moved between widths. That sentence is no longer
        // true and has been corrected there; what is still true — and is the
        // whole correctness argument — is that no value is ever WIDENED,
        // SIGN-EXTENDED or TRUNCATED:
        //
        // - IEEE §11.8.1: a concatenation's operands are SELF-DETERMINED. Each
        //   part is compiled at its own `(width, signed)`, which is exactly what
        //   `eval_concat`'s `self.eval(p)` does.
        // - Every part contributes EXACTLY its own `pw` bits at a compile-time
        //   offset. A part's signedness never causes a fill, because there is no
        //   room to fill: the offsets are laid end to end and `Σ pw == w` is
        //   checked below, so the ranges tile the result exactly.
        // - The accumulator starts as definite ZERO and each splice writes a
        //   DISJOINT range, so merging is a plane-wise OR — the same contract
        //   `eval::eval_core::copy_bits` documents for its zeroed destination.
        //   That is why this is not a 4-state `Or` (which would have to decide
        //   `1|x`): no bit is ever written twice.
        //
        // ORDER, not just value: an admitted subtree can COUNT an out-of-range
        // element read (`LoadIdx`), and those reports are ORDERED (the arena's
        // `pending_range` is a Vec precisely because two counters could not carry
        // order). `eval_concat` evaluates `parts` in SOURCE order — parts[0], the
        // MSB-most, first — so the parts are emitted in source order here too and
        // each carries its own precomputed offset. Emitting LSB-first would have
        // produced identical bits and reversed two diagnostics.
        // ── CONDITIONAL (S2 slice 7) ───────────────────────────────────────
        //
        // ⚠️ THE ADMISSION IS ABOUT LAZINESS, NOT WIDTH. `eval_ctx`'s `Ternary`
        // arm evaluates the condition and then ONLY THE TAKEN BRANCH; it
        // evaluates both solely when the condition is unknown. This module has
        // no control flow, so it evaluates both always — which produces the same
        // VALUE (every admitted op is pure) but not necessarily the same
        // DIAGNOSTICS, because `LoadIdx` COUNTS an out-of-range element read.
        // An untaken branch carrying one would report an access the generic path
        // never performs: a diagnostic APPEARING is a divergence exactly as much
        // as one going missing.
        //
        // So the check is not syntactic and not conservative-by-shape — it asks
        // the compiled OPS whether either branch can report, which is the one
        // question that matters and is exact. `LoadIdx` is the only op in this
        // module that touches `note_bad_index`.
        //
        // The branches are CONTEXT-determined at `(w, signed)` — the same
        // (w, eff_signed) the generic passes down — so uniform admission applies
        // to them unchanged, and a branch narrower than the context declines
        // rather than being silently widened here.
        sim_ir::Expr::Ternary {
            cond,
            then_e,
            else_e,
        } => {
            let cw = wt.get(*cond);
            if cw.width == 0 || cw.width > 64 {
                return None;
            }
            compile_node(
                ir, wt, arena, *cond, cw.width, cw.signed, ops, depth, max_depth,
            )?;
            let branches_at = ops.len();
            compile_node(ir, wt, arena, *then_e, w, signed, ops, depth, max_depth)?;
            compile_node(ir, wt, arena, *else_e, w, signed, ops, depth, max_depth)?;
            if ops[branches_at..]
                .iter()
                .any(|o| matches!(o, WOp::LoadIdx { .. }))
            {
                return None;
            }
            *depth -= 2; // three on the stack, one left
            ops.push(WOp::Tern {
                cw: cw.width,
                cs: cw.signed,
                m: mask,
            });
            Some(())
        }
        // ── BIT / PART SELECT (S2 slice 6) ─────────────────────────────────
        //
        // ADMISSION: a 2-state CONSTANT offset whose range lies WHOLLY inside
        // the base. That is not convenience, it is the correctness argument.
        // `eval_select` has THREE outcomes — all-X when the offset is unknown or
        // outside the i64 lane, one `copy_bits` when the range is fully inside
        // the source, and a per-bit loop that X-fills the overhang otherwise —
        // and proving the range at COMPILE time means only the middle one is
        // reachable. `copy_bits(out, 0, src, lsb, w)` into a zeroed `w`-bit
        // destination IS a shift and a mask, which is the whole `Slice` op.
        //
        // The base is SELF-determined (`eval_select` calls `self.eval(base)`),
        // and the select's own result is UNSIGNED — the entry gate above already
        // required the context to agree, so nothing here re-decides either.
        //
        // What this does NOT admit is an indexed select with a RUNTIME offset
        // (`x[i +: 4]`), which is most of what `+:`/`-:` exist for. That needs a
        // runtime bounds test and the X-fill arm; the constant forms are the
        // ones a `[15:0]` costs, and those are the 73 declines measured on
        // picorv32.
        sim_ir::Expr::Select {
            base,
            offset,
            width,
            kind,
        } => {
            // `width` is a const-expr EDGE (`Add(Sub(msb,lsb),1)`), folded by
            // the same helper `eval_select` and the self-width table both use.
            let sel_w = crate::width::const_u32_of_expr(ir, *width).unwrap_or(1);
            let os = wt.get(*offset);
            let off = match ir.exprs.get(*offset as usize)? {
                sim_ir::Expr::Const { val } => {
                    if os.width == 0 || os.width > 64 {
                        return None;
                    }
                    let (ov, ou) = const_planes(ir, *val, os.width)?;
                    // An x/z offset is the generic path's all-X arm.
                    if ou != 0 {
                        return None;
                    }
                    // The SAME two calls the generic makes, over the same value —
                    // `to_u64` deliberately ignores signedness, so a negative
                    // constant reads as a large positive one and then fails the
                    // in-range test below, exactly as it does there.
                    one_word_value(ov, ou, os.width, os.signed)
                        .to_u64()
                        .and_then(|o| i64::try_from(o).ok())?
                }
                _ => return None,
            };
            let (lsb, bits) = crate::eval::binops::select_lsb_width(*kind, off, sel_w);
            // The folded width and the width table must agree; they are two
            // readings of the same const-expr edge and this module must not pick
            // one when they differ.
            if bits != w {
                return None;
            }
            let bs = wt.get(*base);
            if bs.width == 0 || bs.width > 64 {
                return None;
            }
            // `eval_select`'s own fully-in-range condition, verbatim.
            if !(lsb >= 0 && (lsb as u64) + (w as u64) <= bs.width as u64) {
                return None;
            }
            compile_node(
                ir, wt, arena, *base, bs.width, bs.signed, ops, depth, max_depth,
            )?;
            // `lsb + w <= bs.width <= 64` and `w >= 1`, so the shift is ≤ 63.
            ops.push(WOp::Slice {
                k: lsb as u32,
                m: mask,
            });
            Some(())
        }
        // ⭐ THE SIGN SEAL (`$signed`/`$unsigned`). Not a computation — a stamp.
        //
        // `expr_cast` wraps EVERY size cast's result in one of these, so `8'(e)`
        // carries a `SysFunc` node, and this module's catch-all declined it: an
        // external reviewer's workload took **14,616,553** of them from a source
        // that never writes `$signed`, and a 1.6M-iteration cast loop measured
        // **0.678 s sealed vs 0.480 s unsealed = 29.2% of the run**. The body was
        // never the problem (`codegen able` is the same either way); the seal
        // evicted the EXPRESSION.
        //
        // The generic arm is `a = eval(x); a.signed = <stamp>; a.resize_keep_sign(w,
        // <stamp>)`, and the two halves matter separately:
        //
        // * The OPERAND is evaluated at **its own self width and its own sign**
        //   (`selfwidth.rs`: the seal preserves the operand's width and only flips
        //   the sign attribute). That is why this needs no relaxation of the
        //   uniform-sign gate above — the child is compiled at the sign it already
        //   has, so `$signed(<unsigned expression>)` admits both halves.
        // * The FILL is the STAMP, not the child's sign. `$signed` sign-fills iff
        //   the CONTEXT is signed (`a.signed = eff_signed` before the resize);
        //   `$unsigned` always zero-fills. ⚠️ This is why the arm is written out
        //   instead of routed through `CtxClass::SelfDetermined`, whose rule is
        //   `sw.signed && signed` — that reads the CHILD's sign and would fill
        //   wrongly for `$signed(<unsigned>)` in a signed context.
        //
        // Zero-extension needs no op (every stack value is masked to its width);
        // truncation declines, as it does everywhere in this module.
        sim_ir::Expr::SysFunc {
            which: which @ (sim_ir::SysFuncId::Signed | sim_ir::SysFuncId::Unsigned),
            args,
        } if args.len() == 1 => {
            let xw = wt.get(args[0]);
            if !width_admits(xw.width) || xw.width > w {
                return None;
            }
            compile_node(
                ir, wt, arena, args[0], xw.width, xw.signed, ops, depth, max_depth,
            )?;
            let fill_signed = match which {
                sim_ir::SysFuncId::Signed => signed,
                _ => false,
            };
            if xw.width < w && fill_signed {
                ops.push(WOp::Sext {
                    from: xw.width,
                    to: w,
                });
            }
            Some(())
        }
        sim_ir::Expr::Concat { parts } => {
            // `Σ pw == w` is the tiling proof AND the agreement check between
            // this module's two sources of width (the table, and `eval_concat`'s
            // sum over evaluated part widths). If they disagree — a clamp, a
            // saturating add, an empty concat's `.max(1)` — decline rather than
            // guess which one the generic path will use.
            let mut total: u64 = 0;
            for &p in parts.iter() {
                let pw = wt.get(p).width;
                // A ZERO-width part is legal (`{0{x}}`, IEEE §11.4.12.1) and is
                // declined rather than skipped: `eval_concat` still EVALUATES it,
                // so skipping could drop an E4002 the generic path reports.
                if pw == 0 {
                    return None;
                }
                total += u64::from(pw);
            }
            if total != u64::from(w) {
                return None;
            }
            push(ops, depth, max_depth, WOp::Const { val: 0, unk: 0 });
            let mut pos = w;
            for &p in parts.iter() {
                let ps = wt.get(p);
                pos -= ps.width; // parts[0] is MSB-most (`eval_concat`'s `pos -= v.width`)
                compile_node(ir, wt, arena, p, ps.width, ps.signed, ops, depth, max_depth)?;
                ops.push(WOp::Splice {
                    off: pos,
                    stride: 0,
                    count: 1,
                    m: mask,
                });
                *depth -= 1; // splice: two on the stack, one left
            }
            Some(())
        }
        sim_ir::Expr::Replicate { count, value } => {
            // `count` is a const-expr EDGE, not a literal — folded by the same
            // function `eval_replicate` and the self-width table both use.
            let n = crate::width::const_u32_of_expr(ir, *count)?;
            let vs = wt.get(*value);
            // `{0{x}}` has width 0 and would make this node's own width 0, which
            // `compile`'s entry already refuses at the root; declining here too
            // keeps a NESTED one from reaching `mask_of(0)`, and matches the
            // zero-width concat part above (the operand is still evaluated by
            // `eval_replicate`, so skipping is not equivalent).
            if n == 0 || vs.width == 0 {
                return None;
            }
            if u64::from(n) * u64::from(vs.width) != u64::from(w) {
                return None;
            }
            push(ops, depth, max_depth, WOp::Const { val: 0, unk: 0 });
            // ONE compile, `count` splices — `eval_replicate` evaluates the
            // operand ONCE and copies it, so compiling it `count` times would
            // multiply any `LoadIdx` report by `count`.
            compile_node(
                ir, wt, arena, *value, vs.width, vs.signed, ops, depth, max_depth,
            )?;
            ops.push(WOp::Splice {
                off: 0,
                stride: vs.width,
                count: n,
                m: mask,
            });
            *depth -= 1;
            Some(())
        }
        _ => None,
    }
}

/// A `Const` expression's two planes at width `w` (≤64) — `None` when the
/// literal is wider than one word.
fn const_planes(ir: &SimIr, val_idx: u32, w: u32) -> Option<(u64, u64)> {
    if w > 64 {
        return None;
    }
    let cv = ir.consts.get(val_idx as usize)?;
    // Numeric only: a string or real literal has a different value domain and
    // its own sizing rules — generic path.
    if !matches!(cv.repr, sim_ir::ConstRepr::Numeric) || cv.width > 64 {
        return None;
    }
    let v = *cv.bits.val.first().unwrap_or(&0);
    let u = *cv.bits.unk.first().unwrap_or(&0);
    Some((v, u))
}

impl WProg {
    /// Execute over the arena buffer. `scratch` is the caller's reusable
    /// stack; cleared here, capacity grown once to the compiled depth.
    /// Evaluate — 2-state lane first, canonical lane always behind it.
    ///
    /// ## D2: what the fast lane is, and why it costs no correctness
    ///
    /// A census over the eight benchmark shapes and picorv32 measured how often
    /// an evaluation touches x/z at all: **every one of the eight shapes is
    /// 100% definite, and picorv32 is 90.1% of runs / 91.1% of ops.** The
    /// 4-state machinery in the canonical loop below therefore does work that
    /// nine evaluations in ten cannot use — two planes pushed and popped for
    /// every value, a definite-1/definite-0 dance in `And`/`Or`, a branch in
    /// `Add`/`Sub`, and a shared call handed a plane that is provably zero.
    ///
    /// `run_2s` is the same op sequence over ONE plane. It returns `None` the
    /// moment a leaf brings an unknown — and then this function runs the
    /// canonical loop, from the start, on the same inputs.
    ///
    /// ⚠️ **The fallback IS the canonical implementation, not an approximation
    /// of it.** That is the whole correctness argument and it is worth stating
    /// as such: this slice adds no semantics, no static per-net proof, and no
    /// X-entry trap (ROADMAP §5.1 sketched D2 with all three). A 2-state answer
    /// is returned only when every leaf was definite, and on definite leaves
    /// every admitted op is definite-preserving — which the battery measures
    /// against this very loop rather than assuming.
    ///
    /// ⚠️ Re-running is safe because the fast lane has NO side effects: the one
    /// arm that can report (`LoadIdx` out of range, which counts a deferred
    /// diagnostic) bails BEFORE reporting, so the canonical loop files it
    /// exactly once. Nothing else in either loop touches the arena.
    ///
    /// ⚠️⚠️ And note the shape of this against §5.1-ax, which is the slice
    /// before it: a cheap test in front of an expensive one is safe EXACTLY
    /// WHEN the real thing is still behind it. D1.5 put an approximation at a
    /// boundary with nothing behind it and quietly reached neither evaluator.
    /// Here the approximation is "no unknown so far", and being wrong about it
    /// costs a re-run, not an answer.
    pub(crate) fn run(&self, arena: &NetArena, sc: &mut WScratch) -> W {
        // ⭐ The majority of executions never reach an executor — see [`WFast`]
        // for the measurement and for why the collapse is exact on both lanes.
        match self.fast {
            WFast::Leaf { vi } => {
                let vi = vi as usize;
                return W {
                    val: arena.buf[vi],
                    unk: arena.buf[vi + 1],
                };
            }
            WFast::Lit { val, unk } => return W { val, unk },
            WFast::Prog => {}
        }
        if self.two_state {
            if let Some(val) = self.run_2s(arena, &mut sc.u) {
                return W { val, unk: 0 };
            }
        }
        self.run_4s(arena, &mut sc.w)
    }

    /// The 2-state lane — one plane, `None` on the first unknown.
    ///
    /// Every arm is the canonical arm below with `unk` fixed to zero, and the
    /// three that consult a shared rule STILL CALL IT (`truthiness_word`,
    /// `unary1_word`, the `binops` comparisons), handing it the zero plane. That
    /// is deliberate: a respelling of those rules is the defect class this
    /// module's header names, and the constant plane is something the optimiser
    /// can fold but a second copy of the semantics is not something a reviewer
    /// can.
    fn run_2s(&self, arena: &NetArena, scratch: &mut Vec<u64>) -> Option<u64> {
        let buf = &arena.buf;
        scratch.clear();
        scratch.reserve(self.depth);
        for op in &self.ops {
            match *op {
                WOp::Const { val, .. } => scratch.push(val),
                WOp::Load { vi } => {
                    if buf[vi as usize + 1] != 0 {
                        return None;
                    }
                    scratch.push(buf[vi as usize]);
                }
                // ⚠️ TWO checks, not one: the INDEX being definite (it came
                // from definite ops) does not make the ELEMENT definite. A
                // mutation deleting the element check survived the whole suite
                // — no design here held x in an array element and read it
                // through a runtime index on this backend — and is killed by
                // the `g*` rows of `cli/tests/two_state_lane.rs`.
                WOp::LoadIdx { off, elems, .. } => {
                    let i = scratch.last_mut()?;
                    let idx = crate::eval::word_index_of(Some(*i));
                    if idx >= elems {
                        // ⚠️ BAIL BEFORE REPORTING — `note_bad_index` is the one
                        // side effect in either loop, and the canonical lane is
                        // about to run the same op.
                        return None;
                    }
                    let vi = (off + idx * 2) as usize;
                    if buf[vi + 1] != 0 {
                        return None;
                    }
                    *i = buf[vi];
                }
                WOp::Not { m } => {
                    let a = scratch.last_mut()?;
                    *a = !*a & m;
                }
                WOp::And { .. } => {
                    let b = scratch.pop()?;
                    let a = scratch.last_mut()?;
                    *a &= b;
                }
                WOp::Or { .. } => {
                    let b = scratch.pop()?;
                    let a = scratch.last_mut()?;
                    *a |= b;
                }
                WOp::Xor => {
                    let b = scratch.pop()?;
                    let a = scratch.last_mut()?;
                    *a ^= b;
                }
                WOp::Shl { k, m } => {
                    let a = scratch.last_mut()?;
                    *a = (*a << k) & m;
                }
                WOp::Shr { k } => {
                    let a = scratch.last_mut()?;
                    *a >>= k;
                }
                WOp::Add { m } => {
                    let b = scratch.pop()?;
                    let a = scratch.last_mut()?;
                    *a = a.wrapping_add(b) & m;
                }
                WOp::Sub { m } => {
                    let b = scratch.pop()?;
                    let a = scratch.last_mut()?;
                    *a = a.wrapping_sub(b) & m;
                }
                WOp::LogBin { op, lw, rw, .. } => {
                    let b = scratch.pop()?;
                    let a = scratch.last_mut()?;
                    let (v, u) = crate::eval::binops::log_bin_tri(
                        op,
                        crate::eval::truthiness_word(*a, 0, mask_of(lw)),
                        crate::eval::truthiness_word(b, 0, mask_of(rw)),
                    );
                    if u != 0 {
                        return None;
                    }
                    *a = v;
                }
                WOp::Unary1 { op, ow, .. } => {
                    let a = scratch.last_mut()?;
                    let (v, u) = crate::eval::unary1_word(op, *a, 0, mask_of(ow));
                    if u != 0 {
                        return None;
                    }
                    *a = v;
                }
                WOp::Cmp { op, ow, osigned } => {
                    let b = scratch.pop()?;
                    let a = scratch.last_mut()?;
                    let (v, u) = match op {
                        sim_ir::BinOp::CaseEq | sim_ir::BinOp::CaseNe => {
                            crate::eval::binops::case_eq_word(op, *a, 0, b, 0)
                        }
                        sim_ir::BinOp::Eq | sim_ir::BinOp::Ne => {
                            crate::eval::binops::log_eq_word(op, *a, 0, b, 0)
                        }
                        _ => crate::eval::binops::relational_word(op, *a, 0, b, 0, ow, osigned),
                    };
                    if u != 0 {
                        return None;
                    }
                    *a = v;
                }
                WOp::Tern { cw, .. } => {
                    let e = scratch.pop()?;
                    let t = scratch.pop()?;
                    let c = scratch.last_mut()?;
                    // ⚠️ NO `& m` on the taken branch — the canonical arm has
                    // none either, because a branch value is already masked to
                    // the result width. Masking here would be a SECOND SPELLING
                    // of that invariant: it is a no-op today, and the day the
                    // invariant breaks it would hide the break in this lane
                    // while the canonical lane showed it.
                    *c = match crate::eval::truthiness_word(*c, 0, mask_of(cw)) {
                        crate::eval::Tri::True => t,
                        crate::eval::Tri::False => e,
                        // A definite condition cannot be Unknown; fail closed
                        // rather than argue, since the cost is one re-run.
                        crate::eval::Tri::Unknown => return None,
                    };
                }
                WOp::Sext { from, to } => {
                    // 2-state lane: `unk` is definite 0 here by construction (the
                    // lane bails on any unknown), so the sign fill has no x arm to
                    // reach — the SAME function still decides it.
                    let a = scratch.last_mut()?;
                    *a = crate::value::resize_word(*a, 0, from, to, true).0;
                }
                WOp::Slice { k, m } => {
                    let a = scratch.last_mut()?;
                    *a = (*a >> k) & m;
                }
                WOp::Splice {
                    off,
                    stride,
                    count,
                    m,
                } => {
                    let p = scratch.pop()?;
                    let acc = scratch.last_mut()?;
                    for i in 0..count {
                        // ⚠️ `& m` is MEASURED REDUNDANT and kept anyway. The
                        // compiler checks that the parts tile the result
                        // (`Σ pw == w`) and every part is masked to its own
                        // width, so `p << sh` cannot reach past `w`; an
                        // `assert_eq!((p << sh) & !m, 0)` probe ran the whole
                        // 5,476-test suite without firing. It stays because it
                        // is what the canonical arm does — a lane that drops a
                        // guard its twin keeps is a divergence waiting for the
                        // invariant to move.
                        *acc |= (p << (off + i * stride)) & m;
                    }
                }
            }
        }
        scratch.pop()
    }

    fn run_4s(&self, arena: &NetArena, scratch: &mut Vec<W>) -> W {
        let buf = &arena.buf;
        scratch.clear();
        scratch.reserve(self.depth);
        for op in &self.ops {
            match *op {
                WOp::Const { val, unk } => scratch.push(W { val, unk }),
                WOp::Load { vi } => scratch.push(W {
                    val: buf[vi as usize],
                    unk: buf[vi as usize + 1],
                }),
                WOp::LoadIdx { off, elems, m } => {
                    let i = scratch.last_mut().expect("wprog stack");
                    // The index rule is the SHARED one — `word_index_of` is what
                    // `eval_core`'s `Expr::Signal` arm calls, and it owns the
                    // "x/z or beyond-u32 is not a wrap" decision. Reaching it from
                    // planes rather than a `Value` is the whole point of this op.
                    let clean = if i.unk != 0 { None } else { Some(i.val) };
                    let idx = crate::eval::word_index_of(clean);
                    if idx >= elems {
                        arena.note_bad_index(
                            arena.net_at_off(off),
                            idx == crate::eval::WORD_UNKNOWN,
                        );
                        i.val = 0;
                        i.unk = m;
                    } else {
                        let vi = (off + idx * 2) as usize;
                        i.val = buf[vi];
                        i.unk = buf[vi + 1];
                    }
                }
                WOp::Not { m } => {
                    let a = scratch.last_mut().expect("wprog stack");
                    // 0→1, 1→0, x/z→x: val flips only where definite.
                    a.val = !a.val & m & !a.unk;
                }
                WOp::And { m } => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    // definite-1 = val&!unk (excludes z); definite-0 = !val&!unk.
                    let d1 = (a.val & !a.unk) & (b.val & !b.unk);
                    let d0 = (!a.val & !a.unk) | (!b.val & !b.unk);
                    a.val = d1;
                    a.unk = m & !(d1 | d0);
                }
                WOp::Or { m } => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    let d1 = (a.val & !a.unk) | (b.val & !b.unk);
                    let d0 = (!a.val & !a.unk) & (!b.val & !b.unk);
                    a.val = d1;
                    a.unk = m & !(d1 | d0);
                }
                WOp::Xor => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    let unk = a.unk | b.unk;
                    a.val = (a.val ^ b.val) & !unk;
                    a.unk = unk;
                }
                WOp::Shl { k, m } => {
                    let a = scratch.last_mut().expect("wprog stack");
                    a.val = (a.val << k) & m;
                    a.unk = (a.unk << k) & m;
                }
                WOp::Shr { k } => {
                    let a = scratch.last_mut().expect("wprog stack");
                    a.val >>= k;
                    a.unk >>= k;
                }
                WOp::Add { m } => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    if (a.unk | b.unk) != 0 {
                        a.val = 0;
                        a.unk = m; // any x/z operand ⇒ all-X (IEEE arithmetic)
                    } else {
                        a.val = a.val.wrapping_add(b.val) & m;
                    }
                }
                WOp::Sub { m } => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    if (a.unk | b.unk) != 0 {
                        a.val = 0;
                        a.unk = m;
                    } else {
                        a.val = a.val.wrapping_sub(b.val) & m;
                    }
                }
                WOp::LogBin { op, lw, ls, rw, rs } => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    // Each operand at ITS OWN width — see the compile-side note.
                    // Signedness plays no part in a TRUTH question, which is why
                    // the plane form does not take it.
                    let _ = (ls, rs);
                    let (v, u) = crate::eval::binops::log_bin_tri(
                        op,
                        crate::eval::truthiness_word(a.val, a.unk, mask_of(lw)),
                        crate::eval::truthiness_word(b.val, b.unk, mask_of(rw)),
                    );
                    a.val = v;
                    a.unk = u;
                }
                WOp::Unary1 { op, ow, os } => {
                    let a = scratch.last_mut().expect("wprog stack");
                    // `!` and the six reductions are all one-bit results over a
                    // self-determined operand, and none of them consults its
                    // sign — `unary1_word` is the plane-level entry point of the
                    // very `unary_self_of` this used to call.
                    let _ = os;
                    let (v, u) = crate::eval::unary1_word(op, a.val, a.unk, mask_of(ow));
                    a.val = v;
                    a.unk = u;
                }
                WOp::Cmp { op, ow, osigned } => {
                    let b = scratch.pop().expect("wprog stack");
                    let a = scratch.last_mut().expect("wprog stack");
                    // Hand both operands to the SHARED comparison functions as
                    // ordinary values (≤64 bits ⇒ `Words::Inline`, no
                    // allocation). What is specialized is reaching this point,
                    // not what happens at it.
                    // The planes go straight to the rule. Admission already
                    // guarantees what the general path's clone-and-resize
                    // establishes — both operands share a width and a
                    // signedness, and every stack value is masked to its own
                    // width — so these are the SAME functions, entered one
                    // level lower. Building two 72-byte `Value`s to say so was
                    // measured at 7.3% of a picorv32 run.
                    let (rv, ru) = match op {
                        sim_ir::BinOp::CaseEq | sim_ir::BinOp::CaseNe => {
                            crate::eval::binops::case_eq_word(op, a.val, a.unk, b.val, b.unk)
                        }
                        sim_ir::BinOp::Eq | sim_ir::BinOp::Ne => {
                            crate::eval::binops::log_eq_word(op, a.val, a.unk, b.val, b.unk)
                        }
                        _ => crate::eval::binops::relational_word(
                            op, a.val, a.unk, b.val, b.unk, ow, osigned,
                        ),
                    };
                    a.val = rv;
                    a.unk = ru;
                }
                WOp::Tern { cw, cs, m } => {
                    let e = scratch.pop().expect("wprog stack");
                    let t = scratch.pop().expect("wprog stack");
                    let c = scratch.last_mut().expect("wprog stack");
                    // The condition is SELF-determined and asked with the same
                    // free function every other truth question in this module
                    // uses (`&&`, `||`, `!`, a branch condition).
                    let _ = cs; // the condition is a TRUTH question, not a signed one
                    *c = match crate::eval::truthiness_word(c.val, c.unk, mask_of(cw)) {
                        crate::eval::Tri::True => t,
                        crate::eval::Tri::False => e,
                        crate::eval::Tri::Unknown => {
                            let (v, u) =
                                crate::eval::binops::merge_x_word(t.val, t.unk, e.val, e.unk);
                            // `merge_x_word` X-poisons every DIFFERING bit,
                            // including above `w`; the generic path masks with
                            // `top_mask`, this one with the op's own mask.
                            W {
                                val: v & m,
                                unk: u & m,
                            }
                        }
                    };
                }
                WOp::Sext { from, to } => {
                    let a = scratch.last_mut().expect("wprog stack");
                    let (v, u) = crate::value::resize_word(a.val, a.unk, from, to, true);
                    a.val = v;
                    a.unk = u;
                }
                WOp::Slice { k, m } => {
                    let a = scratch.last_mut().expect("wprog stack");
                    a.val = (a.val >> k) & m;
                    a.unk = (a.unk >> k) & m;
                }
                WOp::Splice {
                    off,
                    stride,
                    count,
                    m,
                } => {
                    let p = scratch.pop().expect("wprog stack");
                    let acc = scratch.last_mut().expect("wprog stack");
                    // `off + i*stride < w <= 64` holds by construction (the
                    // compiler checked that the ranges tile a ≤64-bit result and
                    // refused a zero-width part), so no shift here can reach 64.
                    for i in 0..count {
                        let sh = off + i * stride;
                        acc.val |= (p.val << sh) & m;
                        acc.unk |= (p.unk << sh) & m;
                    }
                }
            }
        }
        scratch.pop().expect("wprog result")
    }

    pub(crate) fn width(&self) -> u32 {
        self.width
    }

    pub(crate) fn signed(&self) -> bool {
        self.signed
    }

    /// D2 teeth: the three questions the lane's correctness rests on, asked
    /// directly rather than through `run`'s dispatch — because a lane that
    /// silently never fires passes every differential in the repository.
    /// `run` with the single-leaf short circuit BYPASSED — the executor the
    /// fast arm skips, so a test can compare the two answers directly.
    #[cfg(test)]
    pub(crate) fn run_ops_for_test(&self, arena: &NetArena, sc: &mut WScratch) -> W {
        if self.two_state {
            if let Some(val) = self.run_2s(arena, &mut sc.u) {
                return W { val, unk: 0 };
            }
        }
        self.run_4s(arena, &mut sc.w)
    }

    /// D2 teeth, extended: WHICH short circuit (if any) this program takes.
    /// `0` = runs the ops, `1` = single `Load`, `2` = single `Const`.
    ///
    /// ⚠️ Anti-vacuity, for the reason the sibling accessor gives: a lane that
    /// silently never fires passes every differential in the repository, and
    /// the differential CANNOT see this one — it returns the same value either
    /// way, by construction.
    #[cfg(test)]
    pub(crate) fn fast_kind(&self) -> u8 {
        match self.fast {
            WFast::Prog => 0,
            WFast::Leaf { .. } => 1,
            WFast::Lit { .. } => 2,
        }
    }

    /// Op count — the teeth for the sign-seal arm.
    ///
    /// ⚠️ The seal compiles to NOTHING, so a differential cannot see whether the
    /// arm fired: sealed and unsealed produce the same value either way, and the
    /// only difference is whether the sealed form reached this module at all.
    /// Comparing the two op counts is the question that has an answer.
    #[cfg(test)]
    pub(crate) fn ops_len(&self) -> usize {
        self.ops.len()
    }

    #[cfg(test)]
    pub(crate) fn two_state_flag(&self) -> bool {
        self.two_state
    }
    #[cfg(test)]
    pub(crate) fn run_2s_for_test(&self, a: &NetArena, s: &mut Vec<u64>) -> Option<u64> {
        self.run_2s(a, s)
    }
    #[cfg(test)]
    pub(crate) fn run_4s_for_test(&self, a: &NetArena, s: &mut Vec<W>) -> W {
        self.run_4s(a, s)
    }

    /// Instruction count — read by the battery to assert that COMPOUND
    /// comparison operands are actually present (a suite in which every
    /// comparison operand is one `Load` cannot see the per-op masks).
    #[cfg(test)]
    pub(crate) fn op_count(&self) -> usize {
        self.ops.len()
    }
}

/// A ≤64-bit 4-state `Value` from the two plane words — the bridge to code
/// that speaks `Value` (the comparison functions today).
fn one_word_value(val: u64, unk: u64, w: u32, signed: bool) -> Value {
    let mut v = Value::zeros(w, signed);
    v.val[0] = val;
    v.unk[0] = unk;
    v
}

/// Which sizing rule an expression follows — the ONE question the width
/// admission above needs and the one it must not guess.
///
/// ⚠️ There is deliberately no catch-all arm mapping to a sizing rule: an
/// `Expr` this module has no compile arm for maps to `Unknown`, which declines,
/// which is exactly what the blanket width gate did before. Getting this wrong
/// in the `SelfDetermined` direction folds an operator at the wrong width and
/// produces a wrong ANSWER (see the `+` example above), so the safe default is
/// to refuse rather than to pick.
enum CtxClass {
    SelfDetermined,
    ContextDetermined,
    Unknown,
}

fn node_ctx_class(ir: &SimIr, eid: u32) -> CtxClass {
    use sim_ir::BinOp as B;
    use sim_ir::UnOp as U;
    match ir.exprs.get(eid as usize) {
        // Leaves and the width-fixing constructors: their value IS their width.
        Some(sim_ir::Expr::Signal { .. })
        | Some(sim_ir::Expr::Const { .. })
        | Some(sim_ir::Expr::Select { .. })
        | Some(sim_ir::Expr::Concat { .. })
        | Some(sim_ir::Expr::Replicate { .. }) => CtxClass::SelfDetermined,
        // One-bit results (§11.4.7 / §11.8.1) — self-determined by definition,
        // and the arms below already compile their operands at their OWN widths.
        // ⚠️ `_`-FREE on purpose (the project's accept-gate rule): a new operator
        // must not inherit a sizing rule by falling into a catch-all. Adding one
        // breaks this build, which is the intended cost.
        Some(sim_ir::Expr::Binary { op, .. }) => match op {
            // One-bit results (§11.8.1 / §11.4.7): self-determined by definition,
            // and the arms below already compile their operands at their OWN widths.
            B::Lt
            | B::Le
            | B::Gt
            | B::Ge
            | B::Eq
            | B::Ne
            | B::CaseEq
            | B::CaseNe
            | B::CasezEq
            | B::CasexEq
            | B::LogAnd
            | B::LogOr => CtxClass::SelfDetermined,
            // §11.6.1: the arithmetic and bitwise operators take the context's
            // width. The shifts are here for their LEFT operand (the count is
            // self-determined, and is a compile-time constant on this path).
            // `Mul`/`Div`/`Mod`/`Pow` have no compile arm — the classification is
            // still the LRM's, and their arm declines as it did before.
            B::Add
            | B::Sub
            | B::Mul
            | B::Div
            | B::Mod
            | B::Pow
            | B::BitAnd
            | B::BitOr
            | B::BitXor
            | B::BitXnor
            | B::Shl
            | B::Shr
            | B::AShl
            | B::AShr => CtxClass::ContextDetermined,
        },
        Some(sim_ir::Expr::Unary { op, .. }) => match op {
            U::LogNot | U::RedAnd | U::RedNand | U::RedOr | U::RedNor | U::RedXor | U::RedXnor => {
                CtxClass::SelfDetermined
            }
            U::BitNot | U::Minus | U::Plus => CtxClass::ContextDetermined,
        },
        // `?:` takes the context on both branches (§11.4.11).
        Some(sim_ir::Expr::Ternary { .. }) => CtxClass::ContextDetermined,
        _ => CtxClass::Unknown,
    }
}
