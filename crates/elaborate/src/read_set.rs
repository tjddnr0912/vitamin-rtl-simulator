//! ENG-2: the implicit read set — the nets an `always_comb` / `always_latch` / static
//! deferred assertion (IEEE 1800-2017 §9.2.2.2.1, §16.4.3) and an `always @*` / in-body
//! `@(*)` (§9.4.2.2) wait on.
//!
//! [`Elaborator::comb_read_set_in`] is the one builder (a frame body's in-body `@(*)`
//! excepted, below). It walks the LOWERED body, so the lowering records here what the IR
//! no longer shows: which expressions, statements and call sites came from an assertion
//! action, an inlined task body, an inlined static function or void-function body, or the
//! implicit lvalue read of a compound assignment.
//! The records are keyed by arena id and live as long as the arenas.
//!
//! The rule, per mode:
//! - `always_comb` / `always_latch` ([`ReadSetMode::Comb`]): every function called within
//!   the block adds its body's reads, transitively, minus its own variables (exception a)
//!   — an expression call, a discarded call, a call through an output formal, a call in an
//!   lvalue index, a void function (inlined or framed), a call nested in a walked body;
//!   a task adds only its argument expressions (:9056-9057); an assertion's action blocks
//!   add nothing (:9061-9068); a read whose longest static prefix (constant selects only)
//!   is covered by what the block writes is dropped, at variable granularity (exception b).
//! - `@*` ([`ReadSetMode::Star`]): the identifiers in the statement (:9590-9597) — a call's
//!   argument expressions, not the inlined body of a static function, void function or
//!   task, and not the implicit read of a compound assignment's lvalue.
//!
//! A mark is a nesting depth per kind, and every exclusion is relative to the read set's
//! root: an in-body `@(*)` inside an inlined static task body reads its own controlled
//! statement (§9.4.2.2), while a task called from that statement still adds nothing.
//!
//! When the marks are made and when the sets are built (ENG-2 r3):
//! - R-SITE: provenance is decided by identity when IR is created, never recovered from a
//!   position or an absent mark. The two arena appends (`push_expr`, `push_stmt`) stamp
//!   the current mark; once lowering ends ([`Elaborator::begin_resolve_phase`]) an append
//!   is legal only inside [`Elaborator::with_site`], under the mark the walk reads for the
//!   lowering-time node the new IR replaces or extends (a placeholder's, a replaced rhs
//!   root's, or the call site's), so a resolve-time builder produces the marks a local
//!   reference would have had. An append outside a site, or after the settle, fails a
//!   debug assertion.
//! - R-SETTLE: every implicit read set (each comb-inferred process header, each in-body
//!   `@(*)`) is computed once, for every design, after the last post-lowering builder
//!   (`recompute_comb_sensitivity_after_hier`); nothing appends after it
//!   ([`Elaborator::seal_read_sets`]).
//! - R-MEMBER: an in-body `@(*)`'s controlled statement is the statements its open block
//!   held when the statement's lowering ended plus what a call inside it splices into that
//!   block after lowering (statement id >= `sid0`) — never a position count.
//!
//! What a write covers (exception b), decided once in the settle by the one write
//! collector the block's own assignments use (`assign_writes` / `bind_writes`):
//! - R1 a call actual covers what the same write spelled in the block covers: `input`
//!   nothing; `output` / `inout` the actual (R2: the copy-out writes it whole on every
//!   path); `const ref` nothing; `ref` the callee's written regions of the formal
//!   ([`Elaborator::build_ref_write_map`], the least fixed point over every body that
//!   names the formal's storage, minus the inline copy-ins) mapped onto the actual by the
//!   copy-out's bit map.
//! - R3 a system task / function output argument (the read-only funnel's write
//!   positions, `rs.sys_dests`) is a write region in `always_comb` / `always_latch`
//!   (:9050 b) and not a read; `@*` reads it (§9.4.2.2).
//! - R5 nothing at lowering decides a cover: W and every read set are computed in the
//!   settle.
//!
//! An in-body `@(*)` in a process body: the lowering builds a cause at the site
//! (`stmt_main.rs`); the settle installs the final set over it and, R4, warns W3056 when
//! that settled set is empty (a hierarchical call's arguments count; a resolved parameter
//! does not).
//!
//! An in-body `@(*)` in a frame body (a frame task, a frame function, a class method;
//! parked, R4) is not this module's: its cause is built at the site by the walk it used
//! before ENG-2 (`Elaborator::frame_star_read_set`) and is final, W3056 decided there from
//! it. The frame classify reads it (`wait_cond_reads_frame_local`) before the resolve
//! passes, which add the reads of a deferred hierarchical operand; settling it would hand
//! the classify's decision a set it never saw.

use super::*;
use std::collections::HashMap;

/// An assertion's action arm (immediate `If { assertion: Some }`, `#0` / `final`).
pub(crate) const MARK_ACTION: u8 = 1;
/// The body of a static task inlined at its call.
pub(crate) const MARK_TASK: u8 = 2;
/// The body of a static function reduced into the calling expression.
pub(crate) const MARK_FN_INLINE: u8 = 4;
/// The body of a static void function inlined at its call (the task machinery).
pub(crate) const MARK_VOID_INLINE: u8 = 8;
/// The lvalue copy a compound assignment (`op=`, `++`, `--`) reads implicitly.
pub(crate) const MARK_COMPOUND: u8 = 16;

/// A provenance mark: how deep inside each kind of region (one per `MARK_*` bit) an
/// expression, statement or call site was lowered.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ReadMark([u8; 5]);

impl ReadMark {
    pub(crate) fn is_empty(self) -> bool {
        self.0 == [0; 5]
    }

    /// One level deeper in every region `kinds` names.
    fn enter(mut self, kinds: u8) -> Self {
        for (i, d) in self.0.iter_mut().enumerate() {
            if kinds & (1 << i) != 0 {
                *d = d.saturating_add(1);
            }
        }
        self
    }

    /// The kinds this mark is deeper in than `root` — the regions entered below the root.
    fn beyond(self, root: ReadMark) -> u8 {
        (0..5).fold(0, |m, i| {
            if self.0[i] > root.0[i] {
                m | (1 << i)
            } else {
                m
            }
        })
    }
}

/// The marks a read set leaves out, relative to its root's mark; `comb` = the
/// `always_comb` rule (writes are recorded, R3 destinations are writes).
#[derive(Debug, Clone, Copy)]
struct Rule {
    ex: u8,
    root: ReadMark,
    comb: bool,
}

impl Rule {
    fn skips(self, m: ReadMark) -> bool {
        m.beyond(self.root) & self.ex != 0
    }

    fn below(self, m: ReadMark, kind: u8) -> bool {
        m.beyond(self.root) & kind != 0
    }
}

/// A `ref` / `const ref` formal (desugared to `inout`): its copy-out is not a write of
/// the caller unless the subroutine writes the formal (§9.2.2.2.1 b), ENG-2 B2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RefFormal {
    Ref,
    ConstRef,
}

/// One write the collector finds: a region, or a `ref` copy-out's region-to-be — the
/// source storage's write regions in `[off, off + w)` mapped onto `dst` (R1).
#[derive(Debug, Clone, Copy)]
enum WriteOf {
    Region(Region),
    Mapped {
        src: u32,
        off: u32,
        w: u32,
        dst: Region,
    },
}

/// Which rule applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadSetMode {
    /// `always_comb`, `always_latch`, a static deferred assertion (§9.2.2.2.1).
    Comb,
    /// `always @*` and an in-body `@(*)` (§9.4.2.2).
    Star,
}

/// Where the frame-call sites of the walked blocks are keyed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CallKeys {
    /// A process body: `task_calls_proc[(proc, block)]`.
    Process(u32),
    /// The global function arena: `task_calls_func[block]`.
    Func,
}

/// An in-body `@(*)`: the `Wait` it patches and the blocks of the statement it controls.
#[derive(Debug, Clone)]
pub(crate) struct StarSite {
    /// The process whose body holds the site (a frame body's site is not recorded).
    pub(crate) proc: u32,
    pub(crate) wait_bb: u32,
    pub(crate) lo: u32,
    pub(crate) hi: u32,
    /// The block open when the controlled statement's lowering ended, and the ids of the
    /// statements it held then: the statements after the `@(*)` append there, and its
    /// terminator is theirs (§9.4.2.2 — the controlled statement only). R-MEMBER: the
    /// controlled statement is these ids plus what a call inside it splices into this block
    /// after lowering (`sid >= sid0`), never a position count.
    pub(crate) open: Option<(u32, Vec<u32>)>,
    /// The mark at the site — the root its exclusions are relative to.
    pub(crate) root: ReadMark,
    /// R4: where and in which instance the site was lowered — W3056 is decided in the
    /// settle, from the settled set, and reported here.
    pub(crate) loc: Option<diag::SourceLoc>,
    pub(crate) ctx: Vec<diag::Frame>,
}

/// R-SITE: when an arena append is legal. Lowering stamps `cur`; after lowering ends an
/// append is legal only inside [`Elaborator::with_site`]; after the read sets are settled,
/// never.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum ArenaPhase {
    #[default]
    Lowering,
    Resolve,
    Sealed,
}

/// The provenance lowering records for [`Elaborator::comb_read_set_in`].
#[derive(Default)]
pub(crate) struct ReadSetMarks {
    /// The mark every expression, statement and call site gets while it is set.
    pub(crate) cur: ReadMark,
    pub(crate) exprs: HashMap<u32, ReadMark>,
    pub(crate) stmts: HashMap<u32, ReadMark>,
    /// A frame call's site mark, keyed like `task_calls_proc` / `pending_task_calls` /
    /// `task_calls_func`.
    pub(crate) proc_calls: BTreeMap<(u32, u32), ReadMark>,
    pub(crate) pending_calls: Vec<(u32, ReadMark)>,
    pub(crate) func_calls: BTreeMap<u32, ReadMark>,
    /// Frame tasks written `function void`.
    pub(crate) void_fids: BTreeSet<u32>,
    /// An inlined static function's result expression → its argument expressions.
    pub(crate) inline_args: HashMap<u32, Vec<u32>>,
    /// …and its aliased dynamic-array actuals (net, with the call site's mark).
    pub(crate) inline_arg_nets: HashMap<u32, Vec<(ReadMark, u32)>>,
    /// The copy-out statements of an inlined task / void function (`caller = formal`):
    /// the formal is the subroutine's own variable, never a read of the caller.
    pub(crate) copy_out: BTreeSet<u32>,
    /// An inlined `inout` / `ref` copy-in (`formal = actual`): the engine's entry copy,
    /// never a write of the formal for W (R1).
    pub(crate) copy_in: BTreeSet<u32>,
    /// An inlined `ref` / `const ref` copy-out statement → (kind, the formal's local).
    pub(crate) ref_copies: HashMap<u32, (RefFormal, u32)>,
    /// A framed `ref` / `const ref` array formal's unpack statement → (FuncId, slot).
    pub(crate) ref_unpacks: HashMap<u32, (u32, u32)>,
    /// Frame subroutines' `ref` / `const ref` formals, by (FuncId, formal slot).
    pub(crate) ref_formals: BTreeMap<(u32, u32), RefFormal>,
    /// The packed temporaries a framed array formal copies out to (unpacked by the
    /// `ref_unpacks` statements): the engine's own write, never gated.
    pub(crate) unpack_temps: BTreeSet<u32>,
    /// R3: every system task / function output argument the lowering passed through the
    /// read-only funnel, by the eid the IR holds.
    pub(crate) sys_dests: BTreeSet<u32>,
    /// R1: the write regions of every `ref` storage net (settle-time, `build_ref_write_map`).
    wmap: HashMap<u32, Vec<Region>>,
    /// The formals and locals of inlined static void functions (exception a).
    pub(crate) void_locals: BTreeSet<u32>,
    /// Every in-body `@(*)` of a process body, for the recompute.
    pub(crate) star_sites: Vec<StarSite>,
    /// R-SITE: the arena phase, whether a site is open, and the statement-arena length
    /// when lowering ended (every later statement is a post-lowering one).
    pub(crate) phase: ArenaPhase,
    pub(crate) in_site: bool,
    pub(crate) sid0: u32,
}

/// One array word of a net: every word (`All`) or one constant word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Word {
    All,
    At(u32),
}

/// A longest-static-prefix region: `bits` `None` is the whole word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Region {
    net: u32,
    word: Word,
    bits: Option<(u32, u32)>,
}

/// What one write covers of its net: the word and the bit range (`None` = whole word).
type Cover = (Word, Option<(u32, u32)>);

#[derive(Default)]
struct Acc {
    reads: BTreeSet<Region>,
    writes: Vec<Region>,
    calls: Vec<u32>,
}

/// Is `r` written whole by `writes` (all on `r.net`)? A write to every word, or to `r`'s
/// word, covers `r` when its bit ranges cover `r`'s; a read of every word is covered only
/// by a write of every word.
fn covered(r: &Region, writes: &[Cover]) -> bool {
    let mut ivs: Vec<(u32, u32)> = Vec::new();
    for &(w, bits) in writes {
        let word_ok = match (w, r.word) {
            (Word::All, _) => true,
            (Word::At(a), Word::At(b)) => a == b,
            (Word::At(_), Word::All) => false,
        };
        if !word_ok {
            continue;
        }
        match bits {
            None => return true,
            Some(iv) => ivs.push(iv),
        }
    }
    let Some((lo, hi)) = r.bits else {
        return false;
    };
    ivs.sort_unstable();
    let mut cur = lo;
    for (a, b) in ivs {
        if a <= cur && b > cur {
            cur = b;
        }
        if cur >= hi {
            return true;
        }
    }
    cur >= hi
}

impl Elaborator<'_> {
    /// Run `f` one level deeper in the regions `kinds` names (`MARK_*` bits; 0 = none).
    pub(crate) fn with_read_mark<R>(&mut self, kinds: u8, f: impl FnOnce(&mut Self) -> R) -> R {
        let saved = self.rs.cur;
        self.rs.cur = saved.enter(kinds);
        let r = f(self);
        self.rs.cur = saved;
        r
    }

    /// Clear the provenance mark for a body lowered on its own (a frame or class-method
    /// body, possibly on demand in the middle of a statement); returns the saved mark.
    pub(crate) fn enter_unmarked_body(&mut self) -> ReadMark {
        std::mem::take(&mut self.rs.cur)
    }

    /// R-SITE: run a post-lowering builder under `mark` — the mark the read-set walk reads
    /// for the lowering-time node the new IR replaces or extends (a placeholder's, a
    /// replaced rhs root's, or a call site's). The only place an append is legal after
    /// lowering ends.
    pub(crate) fn with_site<R>(&mut self, mark: ReadMark, f: impl FnOnce(&mut Self) -> R) -> R {
        let saved = (self.rs.cur, self.rs.in_site);
        self.rs.cur = mark;
        self.rs.in_site = true;
        let r = f(self);
        (self.rs.cur, self.rs.in_site) = saved;
        r
    }

    /// R-SITE: lowering has ended; every append from here on needs a site.
    pub(crate) fn begin_resolve_phase(&mut self) {
        debug_assert!(
            self.rs.cur.is_empty(),
            "R-SITE: a provenance mark is still open when lowering ends: {:?}",
            self.rs.cur
        );
        self.rs.sid0 = self.stmts.len() as u32;
        self.rs.phase = ArenaPhase::Resolve;
    }

    /// R-SETTLE: the read sets are computed; nothing may append after this.
    pub(crate) fn seal_read_sets(&mut self) {
        self.rs.phase = ArenaPhase::Sealed;
    }

    /// R-SITE: is an append legal now? (checked by both arena appends, debug builds).
    #[inline]
    pub(crate) fn rs_append_ok(&self) -> bool {
        match self.rs.phase {
            ArenaPhase::Lowering => true,
            ArenaPhase::Resolve => self.rs.in_site,
            ArenaPhase::Sealed => false,
        }
    }

    /// The mark of a deferred hierarchical call's site, keyed as `call_site` reads it.
    pub(crate) fn hier_call_site_mark(
        &self,
        func_block: Option<u32>,
        proc: u32,
        call_block: u32,
    ) -> ReadMark {
        match func_block {
            Some(fb) => self.rs.func_calls.get(&fb).copied().unwrap_or_default(),
            None => self
                .rs
                .proc_calls
                .get(&(proc, call_block))
                .copied()
                .unwrap_or_default(),
        }
    }

    /// Record a frame subroutine's `ref` / `const ref` formals by slot (B2).
    pub(crate) fn note_ref_formals(&mut self, fid: u32, ports: &[ast::TfPort]) {
        for (slot, p) in ports.iter().enumerate() {
            let k = match p.dir_spelling {
                ast::TfDirSpelling::Ref => RefFormal::Ref,
                ast::TfDirSpelling::ConstRef => RefFormal::ConstRef,
                ast::TfDirSpelling::Declared => continue,
            };
            self.rs.ref_formals.insert((fid, slot as u32), k);
        }
    }

    /// B2: the statements from `s0` on unpack a framed array formal's copy-out into the
    /// caller's array; for a `ref` / `const ref` formal they write only what the callee
    /// writes of the formal (R1; the source range is read from each rhs at the settle).
    pub(crate) fn gate_ref_unpack(&mut self, fid: u32, slot: u32, s0: u32) {
        if self.rs.ref_formals.contains_key(&(fid, slot)) {
            for sid in s0..self.stmts.len() as u32 {
                self.rs.ref_unpacks.insert(sid, (fid, slot));
            }
        }
    }

    /// R1: an inlined `ref` / `const ref` copy-out statement and the formal's local.
    pub(crate) fn note_ref_copy(&mut self, sid: u32, kind: RefFormal, local: u32) {
        self.rs.ref_copies.insert(sid, (kind, local));
    }

    /// R1: an inlined `inout` / `ref` copy-in — not a write of the formal for W.
    pub(crate) fn note_copy_in(&mut self, sid: u32) {
        self.rs.copy_in.insert(sid);
    }

    /// R3: `eid` is a system task / function output argument as the IR holds it.
    pub(crate) fn note_sys_dest(&mut self, eid: u32) {
        self.rs.sys_dests.insert(eid);
    }

    /// Record the provenance of the frame call that just sealed `call_block`, keyed the
    /// way its `TaskCallInfo` is (a process block, or a frame task body's local block).
    pub(crate) fn note_call_site_mark(&mut self, call_block: u32) {
        let m = self.rs.cur;
        if m.is_empty() {
            return;
        }
        if self.frame_task_lowering {
            self.rs.pending_calls.push((call_block, m));
        } else {
            self.rs.proc_calls.insert((self.cur_proc, call_block), m);
        }
    }

    /// `function void` reserved as a frame task: R2.1 walks its body.
    fn walks_like_function(&self, fid: u32) -> bool {
        self.funcs
            .get(fid as usize)
            .is_some_and(|f| !f.is_task || self.rs.void_fids.contains(&fid))
    }

    /// The regions whose reads `mode` leaves out.
    fn excluded_marks(mode: ReadSetMode) -> u8 {
        // :9056-9057 — the contents of a task add nothing (both modes).
        let mut m = MARK_TASK;
        match mode {
            // :9061-9068 — expressions in assertion action blocks add nothing. LRM over
            // both tools (iverilog and verilator add them); owner's criterion (ENG-2 O2).
            ReadSetMode::Comb => m |= MARK_ACTION,
            // :9590-9597 — the identifiers in the statement: no inlined body, no
            // implicit compound-assignment lvalue read.
            ReadSetMode::Star => m |= MARK_FN_INLINE | MARK_VOID_INLINE | MARK_COMPOUND,
        }
        m
    }

    pub(crate) fn mark_of_expr(&self, eid: u32) -> ReadMark {
        self.rs.exprs.get(&eid).copied().unwrap_or_default()
    }

    /// Exact value of a subtree whose every leaf is a known non-negative constant:
    /// `+ - * << >>`, compares, `&& ||` and `?:` — the shapes elaborate emits for a
    /// constant index, including the bounds guard of a packed multi-dimensional element
    /// (`(k >= 0 && k <= 1) ? k*W : 'h80000000`). `None` for anything else.
    fn exact_const(&self, eid: u32) -> Option<u32> {
        fn go(el: &Elaborator<'_>, eid: u32, depth: u32) -> Option<u64> {
            if depth > 64 {
                return None;
            }
            match el.exprs.get(eid as usize)? {
                ir::Expr::Const { val } => {
                    let c = el.consts.get(*val as usize)?;
                    if c.bits.unk.iter().any(|&u| u != 0)
                        || c.bits.val.iter().skip(1).any(|&w| w != 0)
                    {
                        return None;
                    }
                    let v = c.bits.val.first().copied().unwrap_or(0);
                    // A negative signed constant is refused (the arithmetic is unsigned).
                    if c.signed && c.width > 0 && c.width <= 64 && (v >> (c.width - 1)) & 1 == 1 {
                        return None;
                    }
                    Some(v)
                }
                ir::Expr::Ternary {
                    cond,
                    then_e,
                    else_e,
                } => {
                    if go(el, *cond, depth + 1)? != 0 {
                        go(el, *then_e, depth + 1)
                    } else {
                        go(el, *else_e, depth + 1)
                    }
                }
                ir::Expr::Binary { op, lhs, rhs } => {
                    let a = go(el, *lhs, depth + 1)?;
                    let b = go(el, *rhs, depth + 1)?;
                    match op {
                        ir::BinOp::Add => a.checked_add(b),
                        ir::BinOp::Sub => a.checked_sub(b),
                        ir::BinOp::Mul => a.checked_mul(b),
                        ir::BinOp::Shl | ir::BinOp::AShl => {
                            (b < 64 && u64::from(a.leading_zeros()) >= b).then(|| a << b)
                        }
                        ir::BinOp::Shr | ir::BinOp::AShr => (b < 64).then(|| a >> b),
                        ir::BinOp::Ge => Some((a >= b) as u64),
                        ir::BinOp::Gt => Some((a > b) as u64),
                        ir::BinOp::Le => Some((a <= b) as u64),
                        ir::BinOp::Lt => Some((a < b) as u64),
                        ir::BinOp::Eq => Some((a == b) as u64),
                        ir::BinOp::Ne => Some((a != b) as u64),
                        ir::BinOp::LogAnd => Some((a != 0 && b != 0) as u64),
                        ir::BinOp::LogOr => Some((a != 0 || b != 0) as u64),
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        go(self, eid, 0).and_then(|v| u32::try_from(v).ok())
    }

    /// The bit range a select names when its offset (and width) fold exactly.
    fn const_bits(&self, kind: ir::SelKind, offset: u32, width: Option<u32>) -> Option<(u32, u32)> {
        let o = self.exact_const(offset)?;
        let w = match kind {
            ir::SelKind::Bit => 1,
            _ => self.exact_const(width?)?,
        };
        match kind {
            ir::SelKind::PartIdxDown => {
                Some((o.checked_add(1)?.checked_sub(w)?, o.checked_add(1)?))
            }
            _ => Some((o, o.checked_add(w)?)),
        }
    }

    fn read_word(&self, word: Option<u32>) -> Word {
        match word {
            None => Word::All,
            Some(w) => self.exact_const(w).map_or(Word::All, Word::At),
        }
    }

    /// The region a write chunk covers; `None` when its word or bits are not provably
    /// constant — such a write covers nothing (C3).
    fn write_region(&self, ch: &ir::LvalChunk) -> Option<Region> {
        let word = match ch.word {
            None => Word::All,
            Some(w) => Word::At(self.exact_const(w)?),
        };
        let bits = match ch.offset {
            None if ch.width.is_none() => None,
            None => return None,
            Some(o) => Some(self.const_bits(ch.kind, o, ch.width)?),
        };
        Some(Region {
            net: ch.net,
            word,
            bits,
        })
    }

    fn rs_expr(&self, eid: u32, ex: Rule, acc: &mut Acc) {
        let mark = self.mark_of_expr(eid);
        let excl = ex.skips(mark);
        match &self.exprs[eid as usize] {
            ir::Expr::Const { .. } | ir::Expr::ArrayItem { .. } => {}
            ir::Expr::Signal { net, word } => {
                if !excl && !self.is_void_local_read(ex, mark, *net) {
                    acc.reads.insert(Region {
                        net: *net,
                        word: self.read_word(*word),
                        bits: None,
                    });
                }
                if let Some(w) = word {
                    self.rs_expr(*w, ex, acc);
                }
            }
            ir::Expr::Select {
                base,
                offset,
                width,
                kind,
            } => {
                if let ir::Expr::Signal { net, word } = &self.exprs[*base as usize] {
                    let bmark = self.mark_of_expr(*base);
                    if !ex.skips(bmark) && !self.is_void_local_read(ex, bmark, *net) {
                        let w = self.read_word(*word);
                        let bits = if word.is_some() && w == Word::All {
                            None
                        } else {
                            self.const_bits(*kind, *offset, Some(*width))
                        };
                        acc.reads.insert(Region {
                            net: *net,
                            word: w,
                            bits,
                        });
                    }
                    if let Some(x) = word {
                        self.rs_expr(*x, ex, acc);
                    }
                } else {
                    self.rs_expr(*base, ex, acc);
                }
                self.rs_expr(*offset, ex, acc);
                self.rs_expr(*width, ex, acc);
            }
            ir::Expr::Concat { parts } => {
                for &p in parts {
                    self.rs_expr(p, ex, acc);
                }
            }
            ir::Expr::Replicate { count, value } => {
                self.rs_expr(*count, ex, acc);
                self.rs_expr(*value, ex, acc);
            }
            ir::Expr::Unary { operand, .. } => self.rs_expr(*operand, ex, acc),
            ir::Expr::Binary { lhs, rhs, .. } => {
                self.rs_expr(*lhs, ex, acc);
                self.rs_expr(*rhs, ex, acc);
            }
            ir::Expr::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.rs_expr(*cond, ex, acc);
                self.rs_expr(*then_e, ex, acc);
                self.rs_expr(*else_e, ex, acc);
            }
            ir::Expr::SysFunc { which, args } => {
                self.census_sys_func(*which, args);
                for &a in args {
                    self.rs_arg(a, ex, acc);
                }
            }
            // A frame function call: its arguments are read here; its body is walked
            // (always_comb only) through `calls`.
            ir::Expr::Call { func, args } => {
                if !excl {
                    acc.calls.push(*func);
                }
                for &a in args {
                    self.rs_expr(a, ex, acc);
                }
            }
        }
        // An inlined static function's argument expressions are read at the call, whether
        // or not its body reads the formal (:9047, :9595).
        if let Some(args) = self.rs.inline_args.get(&eid) {
            for &a in args {
                self.rs_expr(a, ex, acc);
            }
        }
        for &(m, net) in self.rs.inline_arg_nets.get(&eid).into_iter().flatten() {
            if !ex.skips(m) {
                acc.reads.insert(Region {
                    net,
                    word: Word::All,
                    bits: None,
                });
            }
        }
    }

    /// A system task / function argument. R3: in the `always_comb` rule an output
    /// argument is an lvalue position — its region is a write of the block (unless it
    /// sits in a task body, as a statement's write would) and it is not read; its index
    /// sub-expressions stay reads. `@*` reads it (§9.4.2.2).
    fn rs_arg(&self, a: u32, ex: Rule, acc: &mut Acc) {
        if !(ex.comb && self.rs.sys_dests.contains(&a)) {
            self.rs_expr(a, ex, acc);
            return;
        }
        let mark = self.mark_of_expr(a);
        if !ex.below(mark, MARK_TASK) {
            if let Some(r) = self.sys_dest_region(a) {
                acc.writes.push(r);
            }
        }
        let sig = match &self.exprs[a as usize] {
            ir::Expr::SysFunc {
                which: ir::SysFuncId::ReadmemAssocTarget,
                args,
            } => args.first().copied(),
            _ => Some(a),
        };
        if let Some(ir::Expr::Signal { word: Some(w), .. }) = sig.map(|s| &self.exprs[s as usize]) {
            self.rs_expr(*w, ex, acc);
        }
    }

    /// R3: the region a system output argument writes — a whole variable or a
    /// constant word of it (`$readmem*` into an associative array: the array).
    fn sys_dest_region(&self, a: u32) -> Option<Region> {
        match self.exprs.get(a as usize)? {
            ir::Expr::Signal { net, word } => Some(Region {
                net: *net,
                word: match word {
                    None => Word::All,
                    Some(w) => Word::At(self.exact_const(*w)?),
                },
                bits: None,
            }),
            ir::Expr::SysFunc {
                which: ir::SysFuncId::ReadmemAssocTarget,
                args,
            } => self.sys_dest_region(*args.first()?),
            _ => None,
        }
    }

    /// R3 (record at creation, verify at resolution): every output argument of a
    /// system function the walk meets was recorded by the read-only funnel.
    fn census_sys_func(&self, which: ir::SysFuncId, args: &[u32]) {
        if cfg!(debug_assertions) && !self.had_error {
            for i in sys_func_writer_args(which, args.len()) {
                debug_assert!(
                    self.rs.sys_dests.contains(&args[i]),
                    "R3: system function {which:?} argument {i} is a write position the \
                     read-only funnel did not record"
                );
            }
        }
    }

    fn census_sys_task(&self, which: ir::SysTaskId, args: &[u32]) {
        if cfg!(debug_assertions) && !self.had_error {
            for i in sys_task_writer_args(which, args.len()) {
                debug_assert!(
                    self.rs.sys_dests.contains(&args[i]),
                    "R3: system task {which:?} argument {i} is a write position the \
                     read-only funnel did not record"
                );
            }
        }
    }

    /// Exception a for an inlined static void function: its formals and locals are its
    /// own variables, not reads of the block.
    fn is_void_local_read(&self, ex: Rule, mark: ReadMark, net: u32) -> bool {
        ex.below(mark, MARK_VOID_INLINE) && self.rs.void_locals.contains(&net)
    }

    /// An lvalue: its index expressions are reads (§9.4.2.2 "an index variable on the
    /// left-hand side"); what it writes is the write collector's ([`Self::assign_writes`]).
    fn rs_lval(&self, lv: &ir::Lvalue, ex: Rule, acc: &mut Acc) {
        for ch in &lv.chunks {
            for e in [ch.word, ch.offset, ch.width].into_iter().flatten() {
                self.rs_expr(e, ex, acc);
            }
        }
    }

    fn rs_block(&self, bb: &ir::BasicBlock, ex: Rule, acc: &mut Acc) {
        for &sid in &bb.stmts {
            // :9056-9057 — a write inside a task body is not a write of the block (N26).
            let record = ex.comb
                && !ex.below(
                    self.rs.stmts.get(&sid).copied().unwrap_or_default(),
                    MARK_TASK,
                );
            match &self.stmts[sid as usize] {
                ir::Stmt::BlockingAssign { lhs, rhs }
                | ir::Stmt::NonblockingAssign { lhs, rhs, .. } => {
                    self.rs_lval(lhs, ex, acc);
                    if record {
                        self.assign_writes(sid, lhs, *rhs, &mut |w| {
                            self.take_write(w, &mut acc.writes)
                        });
                    }
                    if !self.rs.copy_out.contains(&sid) {
                        self.rs_expr(*rhs, ex, acc);
                    }
                }
                ir::Stmt::SysTask {
                    which, fmt, args, ..
                } => {
                    self.census_sys_task(*which, args);
                    if let Some(f) = fmt {
                        self.rs_expr(*f, ex, acc);
                    }
                    for &a in args {
                        self.rs_arg(a, ex, acc);
                    }
                }
                ir::Stmt::Disable { .. } => {}
                ir::Stmt::Force { lhs, rhs } => {
                    self.rs_lval(lhs, ex, acc);
                    self.rs_expr(*rhs, ex, acc);
                }
                ir::Stmt::Release { lhs } => self.rs_lval(lhs, ex, acc),
            }
        }
        if let ir::Terminator::Branch { cond, .. } = &bb.term {
            self.rs_expr(*cond, ex, acc);
        }
    }

    /// A frame call's binds: the input actuals and the output actuals' indexes are read;
    /// the output actuals write what [`Self::bind_writes`] says (R1 / R2).
    fn rs_call_binds(&self, info: &TaskCallInfo, site: ReadMark, ex: Rule, acc: &mut Acc) {
        for &(_, eid) in &info.in_binds {
            self.rs_expr(eid, ex, acc);
        }
        let record = ex.comb && !ex.below(site, MARK_TASK);
        for (slot, lv) in &info.out_binds {
            self.rs_lval(lv, ex, acc);
            if record {
                self.bind_writes(info.callee, *slot, lv, &mut |w| {
                    self.take_write(w, &mut acc.writes)
                });
            }
        }
    }

    /// The write collector for one assignment statement — the one definition of "what
    /// this statement writes", shared by the read set (exception b) and W (R1): an
    /// inlined `ref` copy-out writes the callee's regions of the formal mapped onto its
    /// lvalue, a `const ref` one nothing, a framed `ref` array formal's unpack the
    /// regions of its element; every other assignment its lvalue's static regions (C3).
    fn assign_writes(&self, sid: u32, lhs: &ir::Lvalue, rhs: u32, f: &mut impl FnMut(WriteOf)) {
        if let Some(&(kind, local)) = self.rs.ref_copies.get(&sid) {
            if kind == RefFormal::Ref {
                if let Some(dst) = self.single_region(lhs) {
                    let w = self.nets.get(local as usize).map_or(0, |n| n.width);
                    f(WriteOf::Mapped {
                        src: local,
                        off: 0,
                        w,
                        dst,
                    });
                }
            }
            return;
        }
        if let Some(&(fid, slot)) = self.rs.ref_unpacks.get(&sid) {
            if self.rs.ref_formals.get(&(fid, slot)) == Some(&RefFormal::Ref) {
                if let (Some(src), Some((off, hi)), Some(dst)) = (
                    self.ref_slot_net(fid, slot),
                    self.unpack_src_range(rhs),
                    self.single_region(lhs),
                ) {
                    f(WriteOf::Mapped {
                        src,
                        off,
                        w: hi - off,
                        dst,
                    });
                }
            }
            return;
        }
        for ch in &lhs.chunks {
            if let Some(r) = self.write_region(ch) {
                f(WriteOf::Region(r));
            }
        }
    }

    /// The write collector for one framed call out-bind (R1 / R2): an output / inout
    /// copy-out writes its whole actual on every path, a `ref` one the callee's regions
    /// of the formal mapped onto the actual, a `const ref` one nothing; the packed
    /// temporary of an array formal is the engine's own write.
    fn bind_writes(&self, callee: u32, slot: u32, lv: &ir::Lvalue, f: &mut impl FnMut(WriteOf)) {
        let temp = lv
            .chunks
            .iter()
            .all(|c| self.rs.unpack_temps.contains(&c.net));
        match self.rs.ref_formals.get(&(callee, slot)) {
            Some(RefFormal::ConstRef) if !temp => {}
            Some(RefFormal::Ref) if !temp => {
                if let (Some(src), Some(dst)) =
                    (self.ref_slot_net(callee, slot), self.single_region(lv))
                {
                    let w = self.nets.get(src as usize).map_or(0, |n| n.width);
                    f(WriteOf::Mapped {
                        src,
                        off: 0,
                        w,
                        dst,
                    });
                }
            }
            _ => {
                for ch in &lv.chunks {
                    if let Some(r) = self.write_region(ch) {
                        f(WriteOf::Region(r));
                    }
                }
            }
        }
    }

    /// A `ref` actual is one variable or element: its single chunk's static region.
    fn single_region(&self, lv: &ir::Lvalue) -> Option<Region> {
        match lv.chunks.as_slice() {
            [ch] => self.write_region(ch),
            _ => None,
        }
    }

    /// The storage net of frame formal `slot` of `fid`.
    fn ref_slot_net(&self, fid: u32, slot: u32) -> Option<u32> {
        self.func_metas.get(fid as usize).map(|m| m.base_net + slot)
    }

    /// An unpack statement's source range in the packed temporary (`temp[off +: w]`).
    fn unpack_src_range(&self, rhs: u32) -> Option<(u32, u32)> {
        match self.exprs.get(rhs as usize)? {
            ir::Expr::Select {
                base,
                offset,
                width,
                kind,
            } if matches!(
                self.exprs.get(*base as usize),
                Some(ir::Expr::Signal { word: None, .. })
            ) =>
            {
                self.const_bits(*kind, *offset, Some(*width))
            }
            _ => None,
        }
    }

    /// Bits / words that are not addressable as a packed vector: a write covers only
    /// when it writes the whole variable.
    fn non_packed(&self, net: u32) -> bool {
        self.nets.get(net as usize).is_none_or(|n| {
            n.array_len != 1
                || matches!(
                    n.kind,
                    ir::NetKind::Real
                        | ir::NetKind::DynArray
                        | ir::NetKind::Queue
                        | ir::NetKind::Assoc
                        | ir::NetKind::AssocStr
                        | ir::NetKind::String
                )
        })
    }

    /// R1 `M`: the regions `ws` of source storage `src` (its bits `[off, off + w)`)
    /// mapped onto `dst` — intersect, shift, place in `dst`, clip to its width; a
    /// non-packed source covers `dst` only when written whole.
    fn map_cover(
        &self,
        src: u32,
        off: u32,
        w: u32,
        dst: Region,
        ws: &[Region],
        out: &mut Vec<Region>,
    ) {
        let dst_w = match dst.bits {
            Some((a, b)) => b - a,
            None => self.nets.get(dst.net as usize).map_or(0, |n| n.width),
        };
        let dst_flat = dst.word == Word::All
            && self
                .nets
                .get(dst.net as usize)
                .is_some_and(|n| n.array_len != 1);
        let src_np = self.non_packed(src);
        for r in ws {
            let whole = r.word == Word::All && r.bits.is_none();
            if src_np || dst_flat {
                if whole {
                    out.push(dst);
                }
                continue;
            }
            let (lo, hi) = r
                .bits
                .unwrap_or((0, self.nets.get(src as usize).map_or(0, |n| n.width)));
            let (lo, hi) = (lo.max(off), hi.min(off.saturating_add(w)));
            if lo >= hi {
                continue;
            }
            let (lo, hi) = (lo - off, (hi - off).min(dst_w));
            if lo >= hi {
                continue;
            }
            if lo == 0 && hi == dst_w {
                out.push(dst);
                continue;
            }
            let base = dst.bits.map_or(0, |(a, _)| a);
            out.push(Region {
                net: dst.net,
                word: dst.word,
                bits: Some((base + lo, base + hi)),
            });
        }
    }

    /// A collected write, resolved for the read set (a `ref` copy-out through W).
    fn take_write(&self, w: WriteOf, out: &mut Vec<Region>) {
        match w {
            WriteOf::Region(r) => out.push(r),
            WriteOf::Mapped { src, off, w, dst } => {
                if let Some(ws) = self.rs.wmap.get(&src) {
                    self.map_cover(src, off, w, dst, ws, out);
                }
            }
        }
    }

    /// Every R3 destination region inside expression `eid` (a system function's
    /// output arguments; W's scan).
    fn expr_sys_dests(&self, eid: u32, f: &mut impl FnMut(WriteOf)) {
        let mut stack = vec![eid];
        while let Some(e) = stack.pop() {
            match self.exprs.get(e as usize) {
                Some(ir::Expr::SysFunc { args, .. }) => {
                    for &a in args {
                        if self.rs.sys_dests.contains(&a) {
                            if let Some(r) = self.sys_dest_region(a) {
                                f(WriteOf::Region(r));
                            }
                        } else {
                            stack.push(a);
                        }
                    }
                }
                Some(ir::Expr::Select {
                    base,
                    offset,
                    width,
                    ..
                }) => stack.extend([*base, *offset, *width]),
                Some(ir::Expr::Concat { parts }) => stack.extend(parts.iter().copied()),
                Some(ir::Expr::Replicate { count, value }) => stack.extend([*count, *value]),
                Some(ir::Expr::Unary { operand, .. }) => stack.push(*operand),
                Some(ir::Expr::Binary { lhs, rhs, .. }) => stack.extend([*lhs, *rhs]),
                Some(ir::Expr::Ternary {
                    cond,
                    then_e,
                    else_e,
                }) => stack.extend([*cond, *then_e, *else_e]),
                Some(ir::Expr::Call { args, .. }) => stack.extend(args.iter().copied()),
                _ => {}
            }
        }
    }

    /// R1 / R5: W — the write regions of every `ref` storage net (a frame formal's slot,
    /// an inlined formal's local), from the final IR: the write collector over every
    /// block that can name it (process bodies and the function arena; never the bare
    /// arena, so an orphan statement adds nothing), minus the inline copy-ins, closed
    /// over `ref` copy-outs that pass a formal on (least fixed point: recursion adds
    /// only what some finite chain of calls writes). Computed once, in the settle, before
    /// the first read set; nothing when the design has no `ref` storage.
    pub(crate) fn build_ref_write_map(&mut self) {
        let mut storage: BTreeSet<u32> = BTreeSet::new();
        for (&(fid, slot), &k) in &self.rs.ref_formals {
            if k == RefFormal::Ref {
                storage.extend(self.ref_slot_net(fid, slot));
            }
        }
        for &(k, local) in self.rs.ref_copies.values() {
            if k == RefFormal::Ref {
                storage.insert(local);
            }
        }
        if storage.is_empty() {
            self.rs.wmap.clear();
            return;
        }
        let mut w: BTreeMap<u32, BTreeSet<Region>> = BTreeMap::new();
        let mut mapped: Vec<(u32, u32, u32, Region)> = Vec::new();
        {
            let mut add = |x: WriteOf| match x {
                WriteOf::Region(r) if storage.contains(&r.net) => {
                    w.entry(r.net).or_default().insert(r);
                }
                WriteOf::Mapped {
                    src,
                    off,
                    w: sw,
                    dst,
                } if storage.contains(&dst.net) => {
                    mapped.push((src, off, sw, dst));
                }
                _ => {}
            };
            let blocks = (self.processes.iter().enumerate())
                .flat_map(|(p, pr)| {
                    pr.body
                        .iter()
                        .enumerate()
                        .map(move |(b, bb)| (CallKeys::Process(p as u32), b as u32, bb))
                })
                .chain(
                    (self.func_blocks.iter().enumerate())
                        .map(|(b, bb)| (CallKeys::Func, b as u32, bb)),
                );
            for (keys, b, bb) in blocks {
                for &sid in &bb.stmts {
                    if self.rs.copy_in.contains(&sid) {
                        continue;
                    }
                    match &self.stmts[sid as usize] {
                        ir::Stmt::BlockingAssign { lhs, rhs }
                        | ir::Stmt::NonblockingAssign { lhs, rhs, .. } => {
                            self.assign_writes(sid, lhs, *rhs, &mut add);
                            self.expr_sys_dests(*rhs, &mut add);
                        }
                        ir::Stmt::SysTask { args, .. } => {
                            for &a in args {
                                if self.rs.sys_dests.contains(&a) {
                                    if let Some(r) = self.sys_dest_region(a) {
                                        add(WriteOf::Region(r));
                                    }
                                } else {
                                    self.expr_sys_dests(a, &mut add);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                if matches!(bb.term, ir::Terminator::Call { .. }) {
                    if let Some((info, _)) = self.call_site(keys, b) {
                        for (slot, lv) in &info.out_binds {
                            self.bind_writes(info.callee, *slot, lv, &mut add);
                        }
                    }
                }
            }
        }
        // Least fixed point over the `ref` copy-outs (regions are finite: every one is a
        // clipped bit range of a storage net).
        for _ in 0..64 {
            let mut changed = false;
            for &(src, off, sw, dst) in &mapped {
                let ws: Vec<Region> = w
                    .get(&src)
                    .map_or_else(Vec::new, |s| s.iter().copied().collect());
                let mut out = Vec::new();
                self.map_cover(src, off, sw, dst, &ws, &mut out);
                let e = w.entry(dst.net).or_default();
                for r in out {
                    changed |= e.insert(r);
                }
            }
            if !changed {
                break;
            }
        }
        self.rs.wmap = w
            .into_iter()
            .map(|(n, s)| (n, s.into_iter().collect()))
            .collect();
    }

    fn call_site(&self, keys: CallKeys, block: u32) -> Option<(&TaskCallInfo, ReadMark)> {
        match keys {
            CallKeys::Process(p) => Some((
                self.task_calls_proc.get(&(p, block))?,
                self.rs
                    .proc_calls
                    .get(&(p, block))
                    .copied()
                    .unwrap_or_default(),
            )),
            CallKeys::Func => Some((
                self.task_calls_func.get(&block)?,
                self.rs.func_calls.get(&block).copied().unwrap_or_default(),
            )),
        }
    }

    /// The implicit read set of `body`, whose first block is block `base` in the key
    /// space `keys` names; `root` is the mark at its root (zero for a process header, the
    /// site's for an in-body `@(*)`). Deterministic ascending net order.
    pub(crate) fn comb_read_set_in(
        &self,
        keys: CallKeys,
        body: &[ir::BasicBlock],
        base: u32,
        mode: ReadSetMode,
        root: ReadMark,
    ) -> Vec<u32> {
        let ex = Rule {
            ex: Self::excluded_marks(mode),
            root,
            comb: mode == ReadSetMode::Comb,
        };
        let mut acc = Acc::default();
        for (bi, bb) in body.iter().enumerate() {
            self.rs_block(bb, ex, &mut acc);
            if matches!(bb.term, ir::Terminator::Call { .. }) {
                if let Some((info, site)) = self.call_site(keys, base + bi as u32) {
                    self.rs_call_binds(info, site, ex, &mut acc);
                    // A function (or void function) called as a statement — through an
                    // output formal, a hoisted body-write call, a framed void function.
                    if !ex.skips(site) && self.walks_like_function(info.callee) {
                        acc.calls.push(info.callee);
                    }
                }
            }
        }
        if mode == ReadSetMode::Comb {
            self.rs_walk_callees(ex.ex, &mut acc);
        }
        let mut by_net: BTreeMap<u32, Vec<Cover>> = BTreeMap::new();
        for w in &acc.writes {
            by_net.entry(w.net).or_default().push((w.word, w.bits));
        }
        let mut out = BTreeSet::new();
        for r in &acc.reads {
            // Exception b, at variable granularity: a net stays (whole) while any of its
            // reads is not covered by the block's writes.
            if by_net.get(&r.net).is_some_and(|ws| covered(r, ws)) {
                continue;
            }
            out.insert(r.net);
        }
        out.into_iter().collect()
    }

    /// :9047 — the reads of every function called within the block, transitively, minus
    /// the variables each declares (exception a: its frame slots).
    fn rs_walk_callees(&self, ex: u8, acc: &mut Acc) {
        // A frame body is lowered unmarked: its marks are relative to its own root.
        let ex = Rule {
            ex,
            root: ReadMark::default(),
            comb: true,
        };
        let mut visited = BTreeSet::new();
        while let Some(fid) = acc.calls.pop() {
            if !visited.insert(fid) || !self.walks_like_function(fid) {
                continue;
            }
            let Some(f) = self.funcs.get(fid as usize) else {
                continue;
            };
            let lo = self.func_metas.get(fid as usize).map_or(0, |m| m.base_net);
            let hi = lo.saturating_add(f.locals_len);
            let mut body = Acc::default();
            let mut bbs = vec![f.entry];
            let mut seen = BTreeSet::new();
            while let Some(b) = bbs.pop() {
                if !seen.insert(b) {
                    continue;
                }
                let Some(bb) = self.func_blocks.get(b as usize) else {
                    continue;
                };
                self.rs_block(bb, ex, &mut body);
                match &bb.term {
                    ir::Terminator::Goto { target } => bbs.push(*target),
                    ir::Terminator::Branch {
                        then_bb, else_bb, ..
                    } => {
                        bbs.push(*then_bb);
                        bbs.push(*else_bb);
                    }
                    ir::Terminator::Delay { resume, .. } | ir::Terminator::Wait { resume, .. } => {
                        bbs.push(*resume)
                    }
                    ir::Terminator::Fork {
                        children,
                        join,
                        resume_bb,
                    } => {
                        bbs.extend(children.iter().copied());
                        bbs.push(*join);
                        bbs.push(*resume_bb);
                    }
                    // A nested call: its binds are read in this body; its callee is walked.
                    ir::Terminator::Call { target, ret_bb } => {
                        match self.call_site(CallKeys::Func, b) {
                            Some((info, site)) => {
                                self.rs_call_binds(info, site, ex, &mut body);
                                if !ex.skips(site) {
                                    body.calls.push(info.callee);
                                }
                            }
                            None => {
                                if let Some(nf) = self.funcs.iter().position(|g| g.entry == *target)
                                {
                                    body.calls.push(nf as u32);
                                }
                            }
                        }
                        bbs.push(*ret_bb);
                    }
                    ir::Terminator::Return => {}
                }
            }
            let own = |n: u32| n >= lo && n < hi;
            acc.reads
                .extend(body.reads.into_iter().filter(|r| !own(r.net)));
            // Exception b names writes "within any function called within the block" too:
            // a read covered only by a callee's write is dropped. LRM over all three tools
            // (iverilog, sv2v and verilator wake there); owner's criterion (ENG-2 OP1, N25).
            acc.writes
                .extend(body.writes.into_iter().filter(|r| !own(r.net)));
            acc.calls.extend(body.calls);
        }
    }

    /// After lowering a compound assignment (`lv op= e`, `lv++`, `lv--`, desugared by the
    /// parser to `lv = lv op e`): mark the rhs's copy of the lvalue — §9.4.2.2 leaves out an
    /// identifier that appears only as the lvalue. Only a `Signal` naming a net the
    /// statement writes is marked; any other lowered shape keeps every read.
    pub(crate) fn mark_compound_lvalue_copy(&mut self, first_stmt: usize) {
        for sid in first_stmt..self.stmts.len() {
            let ir::Stmt::BlockingAssign { lhs, rhs } = &self.stmts[sid] else {
                continue;
            };
            let written: BTreeSet<u32> = lhs.chunks.iter().map(|c| c.net).collect();
            let mut e = *rhs;
            let copy = loop {
                match &self.exprs[e as usize] {
                    ir::Expr::Select { base, .. } => e = *base,
                    ir::Expr::Binary { lhs, .. } => break Some(*lhs),
                    _ => break None,
                }
            };
            let Some(copy) = copy else {
                continue;
            };
            let mut stack = vec![copy];
            while let Some(x) = stack.pop() {
                match &self.exprs[x as usize] {
                    ir::Expr::Signal { net, .. } if written.contains(net) => {
                        let m = self.rs.exprs.entry(x).or_default();
                        if m.0[4] == 0 {
                            *m = m.enter(MARK_COMPOUND); // one level: it marks no region
                        }
                    }
                    ir::Expr::Select { base, .. } => stack.push(*base),
                    ir::Expr::Concat { parts } => stack.extend(parts.iter().copied()),
                    _ => {}
                }
            }
        }
    }

    /// Re-run the rule for every in-body `@(*)` (its controlled statement may hold a
    /// hierarchical reference patched after lowering — a `POISON_NET` read otherwise),
    /// over exactly the blocks and statements of its controlled statement (R-MEMBER).
    pub(crate) fn recompute_in_body_star_sites(&mut self) {
        let sites = self.rs.star_sites.clone();
        for s in sites {
            let blocks = self
                .processes
                .get(s.proc as usize)
                .and_then(|pr| pr.body.get(s.lo as usize..s.hi as usize));
            let Some(blocks) = blocks else {
                continue;
            };
            let mut body = blocks.to_vec();
            if let Some((ob, ids)) = &s.open {
                if let Some(bb) = ob.checked_sub(s.lo).and_then(|i| body.get_mut(i as usize)) {
                    // R-MEMBER: the statements the block held when the controlled statement's
                    // lowering ended, plus a call's post-lowering splice into it.
                    let sid0 = self.rs.sid0;
                    bb.stmts.retain(|sid| *sid >= sid0 || ids.contains(sid));
                    bb.term = ir::Terminator::Return;
                }
            }
            let keys = CallKeys::Process(s.proc);
            let nets = self.comb_read_set_in(keys, &body, s.lo, ReadSetMode::Star, s.root);
            // R4: W3056 from the settled set, at the site's own place and instance.
            if nets.is_empty() {
                self.sink.emit(LogEvent::Diagnostic(Diagnostic {
                    severity: Severity::Warning,
                    code: MsgCode::ElabFeatureLimit,
                    message: "in-body @(*) reads no nets; it can never wake".to_string(),
                    location: s.loc.clone(),
                    context: s.ctx.clone(),
                    sim_time: None,
                }));
            }
            let blk = self.processes[s.proc as usize]
                .body
                .get_mut(s.wait_bb as usize);
            if let Some(blk) = blk {
                if let ir::Terminator::Wait {
                    cond: ir::WaitCause::Level { nets: old },
                    ..
                } = &mut blk.term
                {
                    *old = nets;
                }
            }
        }
    }
}

/// R3: the output-argument positions of a system function (the read-only funnel's).
fn sys_func_writer_args(which: ir::SysFuncId, n: usize) -> std::ops::Range<usize> {
    use ir::SysFuncId as F;
    let r = match which {
        F::Sscanf | F::Fscanf => 2..n,
        F::Fgets | F::Fread | F::Cast | F::Random => 0..1,
        F::DistUniform
        | F::DistNormal
        | F::DistExponential
        | F::DistPoisson
        | F::DistChiSquare
        | F::DistT
        | F::DistErlang => 0..1,
        F::ValuePlusargs | F::AssocFirst | F::AssocNext | F::AssocLast | F::AssocPrev => 1..2,
        _ => 0..0,
    };
    r.start.min(n)..r.end.min(n)
}

/// R3: the output-argument positions of a system task.
fn sys_task_writer_args(which: ir::SysTaskId, n: usize) -> std::ops::Range<usize> {
    use ir::SysTaskId as T;
    let r = match which {
        T::Cast | T::Sformat => 0..1,
        T::ReadmemB | T::ReadmemH => 1..2,
        _ => 0..0,
    };
    r.start.min(n)..r.end.min(n)
}
