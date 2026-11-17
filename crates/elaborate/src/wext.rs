//! §5.2 Q-WREXT — the write-extent view: which writer writes which statically decided
//! bits of which net, read once over the finished IR. One record (ER §5.1); its home
//! test is `crates/elaborate/tests/wext_home.rs`, its pins `crates/cli/tests/wext_view.rs`.
//!
//! WHAT IT READS. Every chunk of every lvalue the finished IR stores through, in this
//! order: the continuous rows (`cont_assigns`, one append point `push_cont_assign`, the
//! row's kind from `ca_idents`), the process bodies (`processes[p].body`, every block in
//! body order, its statements in block order, the process kind from `proc_idents`, one
//! process append point `push_process`, one statement append point `push_stmt`), then the
//! frame bodies (`funcs[f]`: the blocks of the global `blocks` arena reached from its
//! `entry` through the terminators, in ascending block id). A statement writes through
//! its lvalue when it is one of `stmt_lvalue`'s four: `BlockingAssign`,
//! `NonblockingAssign`, `Force`, `Release` (`Form` keeps which: a reader decides whether
//! a force or a release is a writer, the view does not).
//!
//! THE EXTENT of one chunk, decided here and nowhere else:
//! - `Poison`: the chunk's net id is not a net (`POISON_NET`, a hierarchical-write
//!   sentinel never patched). Only an IR that latched `had_error` keeps one; the finished
//!   IR of a clean run has none (premise judge 1 K7: 0 on 14 real designs).
//! - `Word`: an unpacked-array element (`word` set). Not compared bit-wise here (§5.2
//!   Q-ARRWORDMD owns element extents).
//! - `Bits { lo, hi }`: `[lo, hi)` as `sim_ir::const_chunk_written_bits` answers — the one
//!   home of "which bits a constant chunk writes" (called, not copied: no arithmetic of
//!   the view's own). The net width is the kernel's `width.max(1)`, as
//!   `check_whole_net_multidriver` passes it.
//!   `by: Kernel` names that home ; `by: Geometry` the geometry constant below (C2), for
//!   a packed-array element or an array-of-struct member with constant indices.
//! - `Undecided`: neither decides. That is an offset the kernel does not read — a
//!   run-time index, a constant tree with a multiply, a cast, a conditional or a call
//!   (§5.2 Q-MDCONSTOFF's list: a packed element's `flatten_word` stride product and its
//!   bounds guard are such trees) that the geometry constant did not record either (a
//!   genvar EXPRESSION index such as `2*g+1`: reach, not soundness, premise judge 1 N5)
//!   — or a chunk that writes no bit at all (wholly out of range: the kernel answers
//!   `None` for both, and so does the view). `Undecided` is not a fact: it never stands
//!   for "the whole net".
//!
//! THE WRITER is recorded as the IR has it (premise judge 1 K5): a static task inlined
//! into its caller writes through statements of the CALLER's process, so its writes are
//! that process's (`Writer::Proc`) ; a function, or a task lowered to a frame, writes in
//! its frame body (`Writer::Body`), and the view does not attribute a frame body to the
//! processes that call it. A declaration initializer arrives as the synthesized
//! `var_init` process, a `wire` initializer as a `net_init` row, an instance port hookup
//! as a `port` row, a gate primitive as an `assign` row: the kind string is the one the
//! producer pushed, unedited.
//!
//! OUT — channels that store into a net without an lvalue chunk the view reads (premise
//! judge 1 K1, measured): a frame call's copy-out `out_binds` (q8_task_auto_2comb: no
//! write on the automatic task's actuals), a system task's output arguments (`$fread`,
//! `$sscanf`, `$value$plusargs`: plain expression arguments), the clocking outputs
//! (`clocking_outputs`), an `inout` actual (`inout_actual_exprs`) and the task enables
//! kept for a process (`task_calls_proc`). A reader that needs them reads them beside
//! the view (`cont_array.rs`'s `other_writers` does today).
//!
//! NOT HERE: no target set (which nets a rule looks at: declaration facts, §5.2
//! Q-MDRVMIX's re-plan; no `intro_kind` read), no pair test (which two writes overlap is
//! the reader's). Its one production reader is `mdrv_mix.rs` (§5.2 Q-MDRVMIX: the target
//! set from the declaration facts, the pair test, rule X), through one entry,
//! `check_mixed_drivers`, right after the sole-writer checks. The three checkers that answer parts of this
//! today (`multidriver.rs` rules A-D over the AST, `netdecl.rs`
//! `check_whole_net_multidriver` over continuous rows, `cont_array.rs` `other_writers`)
//! are not re-pointed: their re-point is filed with the readers (§5.2 Q-WREXT). The
//! record is not serialized (no IR field, no `.vu` / `.velab` byte, no `SchemaHash`).
//!
//! WHEN IT IS BUILT. Once per elaboration, in the end phase (`driver.rs`,
//! `build_write_view` right before `check_whole_net_multidriver`): every deferred
//! hierarchical write is patched by then and no array the view reads is appended after it
//! (the sole-writer checks, `check_readmem_assoc_waits`, then api.rs's
//! `assert_block_local_inits_drained`, `finish_assert_item_gate` and `edge_backstop` only
//! read them; `finish` moves them), so the record describes the finished IR. It goes to
//! `Sidecars::write_view` (the test and dump entry) ; in a `vita` run its one reader is
//! `check_mixed_drivers` (`mdrv_mix.rs`), in the same end phase.
//! Built eagerly because it is cheap (Q-WREXT build r1, the view's instructions over an
//! elaboration alone, same release binary, median of 3: +0.12 % y_wext_procs, +0.27 %
//! y_wext_mix, +0.11 % y_wext_port, +0.04 % el2w; W/wrext/build/cost/probe/VIEW_COST.txt):
//! the row that reads it adds only its reader's cost.
//!
//! C2 (§5.2 Q-WREXT C2) adds three records the lowering writes while it decides them,
//! each through one writer fn in this file (`wext_rec`, `WextRecords`), none serialized:
//!
//! THE GEOMETRY CONSTANT (plan L2). A packed-array element, an array-of-struct member and
//! a part of either lower their offset through `flatten_word` / `flatten_word_eids`
//! (`array_geom.rs`): a row-major `coord * stride` sum under a per-dimension bounds guard
//! (`cond ? flat : GEOM_OOB`), a tree the kernel does not read. When every SEALED index is
//! one numeric constant leaf without x / z (`const_index_value`) whose value is in
//! `[0, 2^31)`, `GeomAcc` follows the same arithmetic the lowering emits — the
//! coordinate (`idx - lo`, or `hi - idx` on an ascending dimension), the stride, the guard
//! (`lo <= idx <= hi` on every dimension) — and `record_geom` keeps, keyed by the offset
//! ExprId the twin returns, the value the engine computes for that tree: the flat bit
//! offset, or `GEOM_OOB` when a guard fails. `carry_geom` keeps it through the one
//! composition a write takes on top (`packed_lval.rs`'s `base + l`, the member and the
//! nested part-select of an element). Only a twin call with two or more dimensions
//! records (one dimension emits no guard and no multiply: the kernel reads it) and an
//! array-WORD index never does (a `word` is not a bit extent). A chunk the kernel declines
//! and whose offset ExprId holds a record gets `Bits { by: Geometry }` from
//! `sim_ir::chunk_written_bits_at` (the kernel's width and `sel_written_bits`, so a
//! `GEOM_OOB` offset writes no bit: `Undecided`). Not recorded, so still `Undecided`: a
//! run-time index, a SEALED index the seal wrapped (a narrow signed or an unsigned narrow
//! index becomes a concatenation or an extension), a negative or `>= 2^31` index, an
//! ascending dimension with a negative low bound (refused by the lowering), and a genvar
//! EXPRESSION index (`2*g+1` lowers to a constant tree, not a leaf: premise judge 1 N5 —
//! this home folds leaves only, because a leaf is the one value the engine reads as it is;
//! folding the tree would be a second fold of `Mul` beside the kernel's). Equality with the
//! engine: `sim-engine`'s `geometry_constant_is_what_the_stores_write` writes every
//! geometry-decided chunk of a grid through both stores and compares. REVIEW-ONLY: a later
//! in-place edit of a recorded ExprId's tree (`param_query.rs`'s `patch_expr_param_const*`
//! and `hier_defer/read.rs` write deferred READ placeholders, which are `Signal` nodes when
//! the record is taken, so no constant leaf; `edge_gate.rs` repoints constant EDGES to a
//! fresh id the record does not hold, so the kernel reads the fresh constant).
//!
//! THE DECLARATION FACT (PLAN2 addendum A1). `decl(net)` is `Some(DeclFact { site, kind,
//! dir, type_param })` only for a net created by one of three declaration lowerings, each
//! passing what it knows: `elaborate_netvar_decl_inner`'s integral `add_net` with the
//! caller's `DeclScope` (instance.rs module body -> `Module`, generate.rs -> `Generate`;
//! an interface body, a package and a hoisted block local write no fact), `elaborate_ports`
//! for a module's ANSI header (`AnsiPort`; an interface's ANSI header writes none) and
//! `elaborate_instance`'s non-ANSI `PortDecl` loop (`NonAnsiPort`). `kind` is the
//! declared keyword after `shape_kind` (not `ir::NetKind`), `dir` the net's direction,
//! `type_param` whether a type parameter types it. `record_decl_fact` writes only when
//! the `add_net` just before it pushed a net (a refused duplicate keeps the first
//! declaration's net and writes nothing). Every other net (task / function formals and
//! locals, temporaries, loop variables, implicit nets, string / handle declarations) has
//! `None`. No `intro_kind` and no net name is read. Q-NETSHAPE: the declared shape of a
//! net is that open record row's; this fact is the part a target set reads today, kept
//! here until that row owns it.
//!
//! HOW A STATEMENT CAME TO SIT WHERE IT IS (PLAN2 addendum A2). `record_via` keeps a
//! statement-id interval `[start, stmts.len())` (ids are appended by the one funnel
//! `push_stmt`, in order) for two lowerings: `inline_task` from its INLINE decision to its
//! return (`TaskInline`: copy-in, the body, copy-out and every temporary in between; the
//! method statements, the deferred hierarchical enable and the frame call it dispatches
//! first are not inlines) and the `DeferredAssert` arm around its action arms
//! (`DeferAction`). A procedural write's `via` is `DeferAction` when its statement lies
//! in a `DeferAction` interval, else `TaskInline` when it lies in a `TaskInline` one,
//! else `Direct`; a continuous row has none. The interval says when the statement was
//! pushed, not which body it went to: a statement pushed while a task inlines but placed
//! in another body (a hoisted initializer) carries `TaskInline` there too (PLAN2 R8).
//! ENG-2's `rs.stmts` (`read_set.rs`) keeps a per-statement mark for the read set
//! (another purpose: its task mark covers the body, not the copy-in / copy-out, and its
//! action mark both assertion kinds); not read here.

use super::*;

/// Who writes: the IR's own identity of the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Writer {
    /// Continuous row `row` of `cont_assigns`; `kind` is `ca_idents[row].kind`.
    Cont { row: u32, kind: &'static str },
    /// A statement in process `pid`'s body; `kind` is `proc_idents[pid].kind`.
    Proc { pid: u32, kind: &'static str },
    /// A statement in frame body `funcs[func]`; `task` is its `is_task`.
    Body { func: u32, task: bool },
}

/// How the store writes: a continuous row, or the statement that carries the lvalue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Form {
    Continuous,
    Blocking,
    NonBlocking,
    Force,
    Release,
}

/// Which home decided a `Bits` extent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum By {
    /// `sim_ir::const_chunk_written_bits` read the offset tree.
    Kernel,
    /// The geometry constant recorded for the offset ExprId (module head, C2).
    Geometry,
}

/// Which bits of the net one chunk writes, as far as the finished IR decides it statically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Extent {
    /// Storage bits `[lo, hi)`, `lo < hi`.
    Bits { lo: u32, hi: u32, by: By },
    /// An unpacked-array element: not compared bit-wise.
    Word,
    /// The kernel does not decide it (or it writes no bit).
    Undecided,
    /// The net id is not a net (an IR that latched `had_error`).
    Poison,
}

/// How a procedural statement came to sit in its body (module head, A2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Via {
    /// Lowered from the body's own text.
    Direct,
    /// Pushed while a static task call was inlined (copy-in, body, copy-out).
    TaskInline,
    /// Pushed while a deferred assertion's action arms were lowered.
    DeferAction,
}

/// Where a declaration fact was recorded (module head, A1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeclSite {
    /// A module-body data declaration.
    Module,
    /// A generate-scope data declaration.
    Generate,
    /// A module's ANSI header port.
    AnsiPort,
    /// A module's non-ANSI `PortDecl` with no body declaration of its name.
    NonAnsiPort,
}

/// The scope a data declaration is lowered in, as `elaborate_netvar_decl`'s caller knows
/// it (and `elaborate_ports`': `Module` or `Iface`). Only `Module` and `Generate` record a
/// fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeclScope {
    Module,
    Generate,
    Iface,
    Package,
    BlockLocal,
}

impl DeclScope {
    /// The fact's site for a body declaration in this scope.
    pub(crate) fn body_site(self) -> Option<DeclSite> {
        match self {
            DeclScope::Module => Some(DeclSite::Module),
            DeclScope::Generate => Some(DeclSite::Generate),
            DeclScope::Iface | DeclScope::Package | DeclScope::BlockLocal => None,
        }
    }
}

/// What a declaration lowering knew about the net it created (module head, A1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeclFact {
    pub site: DeclSite,
    /// The declared keyword after `shape_kind` (an omitted port kind is `Wire`).
    pub kind: ast::NetVarKind,
    pub dir: ir::PortDir,
    /// A type parameter types the declaration (`shape_param` is set).
    pub type_param: bool,
}

/// The value the lowering emits for a failed bounds guard (`flatten_word`'s twins): far
/// above any net, so a store at it writes nothing.
pub(crate) const GEOM_OOB: u32 = 0x8000_0000;

/// The records the lowering writes for the view (module head, C2), each only through its
/// writer fn below. Taken by `build_write_view`.
#[derive(Debug, Default)]
pub(crate) struct WextRecords {
    /// Offset ExprId -> the value the engine computes for it (`record_geom`, `carry_geom`).
    geom: std::collections::HashMap<u32, u32>,
    /// Per net id (`record_decl_fact`).
    decls: Vec<Option<DeclFact>>,
    /// `(via, start, end)` statement-id intervals (`record_via`).
    vias: Vec<(Via, u32, u32)>,
}

/// `flatten_word`'s arithmetic over constant leaves, one index at a time (module head).
pub(crate) struct GeomAcc {
    live: bool,
    seen: bool,
    inb: bool,
    flat: u64,
}

impl GeomAcc {
    /// A twin call over `dims` dimensions; `words` = the array-WORD domain.
    pub(crate) fn new(dims: usize, words: bool) -> Self {
        GeomAcc {
            live: dims >= 2 && !words,
            seen: false,
            inb: true,
            flat: 0,
        }
    }

    /// Index `k`: its sealed leaf's value (`None` = not a constant leaf), the dimension's
    /// `(lo, size)`, its direction and stride.
    pub(crate) fn index(&mut self, v: Option<i64>, lo: i64, size: u32, asc: bool, stride: u64) {
        if !self.live {
            return;
        }
        let v = match v {
            Some(v) if (0..=i64::from(i32::MAX)).contains(&v) => v,
            _ => {
                self.live = false;
                return;
            }
        };
        if (asc && lo < 0) || size == 0 || stride > u64::from(u32::MAX) {
            self.live = false;
            return;
        }
        self.seen = true;
        let hi = lo + i64::from(size) - 1;
        if v < lo || v > hi {
            self.inb = false;
            return;
        }
        let coord = if asc { hi - v } else { v - lo } as u64;
        self.flat = self.flat.saturating_add(coord.saturating_mul(stride));
    }

    /// The value the engine computes for the twin's result, when decided.
    pub(crate) fn value(&self) -> Option<u32> {
        if !self.live || !self.seen {
            return None;
        }
        if !self.inb {
            return Some(GEOM_OOB);
        }
        u32::try_from(self.flat)
            .ok()
            .filter(|&f| f <= i32::MAX as u32)
    }
}

/// One write site: one chunk of one lvalue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Write {
    pub writer: Writer,
    pub form: Form,
    /// The statement id (`None` for a continuous row).
    pub stmt: Option<u32>,
    /// How the statement came to sit there (`None` for a continuous row).
    pub via: Option<Via>,
    /// The chunk's index in its lvalue.
    pub chunk: u32,
    /// The chunk's net id as the IR has it (`Poison` when it is not a net).
    pub net: u32,
    pub extent: Extent,
}

/// The view: every write site, in the order the module head names, and the declaration
/// facts per net.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WriteView {
    pub writes: Vec<Write>,
    decls: Vec<Option<DeclFact>>,
}

impl WriteView {
    /// The declaration fact of net `net` (module head, A1).
    pub fn decl(&self, net: u32) -> Option<DeclFact> {
        self.decls.get(net as usize).copied().flatten()
    }
}

/// The arrays the view reads, borrowed (the elaborator's own, or a hand-built set in
/// this file's unit test).
pub(crate) struct WextSource<'a> {
    pub(crate) nets: &'a [ir::NetVar],
    pub(crate) exprs: &'a [ir::Expr],
    pub(crate) consts: &'a [ir::ConstVal],
    pub(crate) cont_assigns: &'a [ir::ContAssign],
    pub(crate) ca_idents: &'a [ProcIdent],
    pub(crate) processes: &'a [ir::Process],
    pub(crate) proc_idents: &'a [ProcIdent],
    pub(crate) stmts: &'a [ir::Stmt],
    pub(crate) blocks: &'a [ir::BasicBlock],
    pub(crate) funcs: &'a [ir::FuncDef],
    pub(crate) geom: &'a std::collections::HashMap<u32, u32>,
    pub(crate) vias: &'a [(Via, u32, u32)],
}

impl Elaborator<'_> {
    /// The end phase's one build (see the module head): the view over the elaborator's
    /// own arrays, kept for `Sidecars::write_view`.
    pub(crate) fn build_write_view(&mut self) {
        let rec = std::mem::take(&mut self.wext_rec);
        let mut view = build_view(&WextSource {
            nets: &self.nets,
            exprs: &self.exprs,
            consts: &self.consts,
            cont_assigns: &self.cont_assigns,
            ca_idents: &self.ca_idents,
            processes: &self.processes,
            proc_idents: &self.proc_idents,
            stmts: &self.stmts,
            blocks: &self.func_blocks,
            funcs: &self.funcs,
            geom: &rec.geom,
            vias: &rec.vias,
        });
        view.decls = rec.decls;
        self.wext = view;
    }

    /// The geometry constant's writer (module head): `eid` is the offset ExprId a
    /// `flatten_word` twin returns, `acc` its arithmetic over the same indices.
    pub(crate) fn record_geom(&mut self, eid: u32, acc: &GeomAcc) {
        if let Some(v) = acc.value() {
            self.wext_rec.geom.insert(eid, v);
        }
    }

    /// The geometry constant through `eid = base + add` (`add` a constant the lowering
    /// pushed beside it): the engine's 32-bit sum, kept when `base` holds a record.
    pub(crate) fn carry_geom(&mut self, eid: u32, base: u32, add: u32) {
        if let Some(v) = self
            .wext_rec
            .geom
            .get(&base)
            .and_then(|v| v.checked_add(add))
        {
            self.wext_rec.geom.insert(eid, v);
        }
    }

    /// The declaration fact's writer (module head, A1): `net` is the id `add_net` gives
    /// the declaration it was just handed (`nets.len()` before the call); nothing is
    /// written when that call created no net or `site` is `None`.
    pub(crate) fn record_decl_fact(
        &mut self,
        net: u32,
        site: Option<DeclSite>,
        kind: ast::NetVarKind,
        dir: ir::PortDir,
        type_param: bool,
    ) {
        let Some(site) = site else {
            return;
        };
        if net as usize >= self.nets.len() {
            return;
        }
        let decls = &mut self.wext_rec.decls;
        if decls.len() <= net as usize {
            decls.resize(net as usize + 1, None);
        }
        decls[net as usize] = Some(DeclFact {
            site,
            kind,
            dir,
            type_param,
        });
    }

    /// The statement-id interval's writer (module head, A2): `[start, stmts.len())`.
    pub(crate) fn record_via(&mut self, via: Via, start: usize) {
        let end = self.stmts.len();
        if start < end {
            self.wext_rec.vias.push((via, start as u32, end as u32));
        }
    }
}

/// The one builder.
pub(crate) fn build_view(s: &WextSource<'_>) -> WriteView {
    let ctx = ir::selfwidth::ExprCtx {
        exprs: s.exprs,
        consts: s.consts,
        nets: s.nets,
    };
    // `via` per statement id, when any interval was recorded: TaskInline first, then
    // DeferAction over it (the module head's order of precedence).
    let mut via_of: Vec<Via> = Vec::new();
    if !s.vias.is_empty() {
        via_of = vec![Via::Direct; s.stmts.len()];
        for want in [Via::TaskInline, Via::DeferAction] {
            for &(v, a, b) in s.vias.iter().filter(|r| r.0 == want) {
                let b = (b as usize).min(via_of.len());
                for slot in via_of.iter_mut().take(b).skip(a as usize) {
                    *slot = v;
                }
            }
        }
    }
    let via = |sid: u32| via_of.get(sid as usize).copied().unwrap_or(Via::Direct);
    let mut writes = Vec::new();
    let mut add = |writer: Writer, form: Form, stmt: Option<u32>, lv: &ir::Lvalue| {
        for (k, c) in lv.chunks.iter().enumerate() {
            writes.push(Write {
                writer,
                form,
                stmt,
                via: stmt.map(via),
                chunk: k as u32,
                net: c.net,
                extent: chunk_extent(ctx, s.geom, c),
            });
        }
    };
    for (row, ca) in s.cont_assigns.iter().enumerate() {
        let kind = s.ca_idents.get(row).map_or("", |id| id.kind);
        let w = Writer::Cont {
            row: row as u32,
            kind,
        };
        add(w, Form::Continuous, None, &ca.lhs);
    }
    for (pid, p) in s.processes.iter().enumerate() {
        let kind = s.proc_idents.get(pid).map_or("", |id| id.kind);
        let w = Writer::Proc {
            pid: pid as u32,
            kind,
        };
        for bb in &p.body {
            for &sid in &bb.stmts {
                if let Some((form, lv)) = store_of(s.stmts, sid) {
                    add(w, form, Some(sid), lv);
                }
            }
        }
    }
    for (f, fd) in s.funcs.iter().enumerate() {
        let w = Writer::Body {
            func: f as u32,
            task: fd.is_task,
        };
        for b in frame_blocks(s.blocks, fd.entry) {
            for &sid in &s.blocks[b as usize].stmts {
                if let Some((form, lv)) = store_of(s.stmts, sid) {
                    add(w, form, Some(sid), lv);
                }
            }
        }
    }
    WriteView {
        writes,
        decls: Vec::new(),
    }
}

/// The form and lvalue of a storing statement (`stmt_lvalue`'s four), else `None`.
fn store_of(stmts: &[ir::Stmt], sid: u32) -> Option<(Form, &ir::Lvalue)> {
    match stmts.get(sid as usize)? {
        ir::Stmt::BlockingAssign { lhs, .. } => Some((Form::Blocking, lhs)),
        ir::Stmt::NonblockingAssign { lhs, .. } => Some((Form::NonBlocking, lhs)),
        ir::Stmt::Force { lhs, .. } => Some((Form::Force, lhs)),
        ir::Stmt::Release { lhs } => Some((Form::Release, lhs)),
        ir::Stmt::SysTask { .. } | ir::Stmt::Disable { .. } => None,
    }
}

/// The blocks of one frame body: every block of the global arena reached from `entry`
/// through the terminators, ascending.
fn frame_blocks(blocks: &[ir::BasicBlock], entry: u32) -> BTreeSet<u32> {
    let mut seen = BTreeSet::new();
    let mut todo = vec![entry];
    while let Some(b) = todo.pop() {
        let Some(bb) = blocks.get(b as usize) else {
            continue;
        };
        if !seen.insert(b) {
            continue;
        }
        match &bb.term {
            ir::Terminator::Goto { target } => todo.push(*target),
            ir::Terminator::Branch {
                then_bb, else_bb, ..
            } => {
                todo.push(*then_bb);
                todo.push(*else_bb);
            }
            ir::Terminator::Delay { resume, .. } | ir::Terminator::Wait { resume, .. } => {
                todo.push(*resume)
            }
            ir::Terminator::Fork {
                children,
                resume_bb,
                ..
            } => {
                todo.extend(children.iter().copied());
                todo.push(*resume_bb);
            }
            ir::Terminator::Call { ret_bb, .. } => todo.push(*ret_bb),
            ir::Terminator::Return => {}
        }
    }
    seen
}

/// One chunk's extent (the module head's four cases, in that order; the kernel before the
/// geometry constant).
fn chunk_extent(
    ctx: ir::selfwidth::ExprCtx<'_>,
    geom: &std::collections::HashMap<u32, u32>,
    c: &ir::LvalChunk,
) -> Extent {
    let Some(nv) = ctx.nets.get(c.net as usize) else {
        return Extent::Poison;
    };
    if c.word.is_some() {
        return Extent::Word;
    }
    let net_w = nv.width.max(1);
    if let Some((lo, hi)) = ir::const_chunk_written_bits(ctx, c, net_w) {
        return Extent::Bits {
            lo,
            hi,
            by: By::Kernel,
        };
    }
    let geo = c
        .offset
        .and_then(|e| geom.get(&e))
        .and_then(|&v| ir::chunk_written_bits_at(ctx, c, i128::from(v), net_w));
    match geo {
        Some((lo, hi)) => Extent::Bits {
            lo,
            hi,
            by: By::Geometry,
        },
        None => Extent::Undecided,
    }
}

impl WriteView {
    /// The dump: one line per write site in the view's order, then one `N` line per net
    /// the view names (ascending id). Text, deterministic: a site's location and a net's
    /// name come from the sidecars of the same elaboration.
    pub fn render(&self, ir: &ir::SimIr, sc: &Sidecars) -> String {
        use std::fmt::Write as _;
        let name = |n: u32| {
            sc.net_names
                .get(n as usize)
                .cloned()
                .unwrap_or_else(|| format!("#{n}"))
        };
        let loc = |id: Option<&ProcIdent>| match id {
            Some(i) if i.line > 0 => format!("@{}:{}:{}", i.file, i.line, i.col),
            _ => String::new(),
        };
        let mut out = String::new();
        let (mut bits, mut geo, mut word, mut und, mut poi) = (0, 0, 0, 0, 0);
        for w in &self.writes {
            match w.extent {
                Extent::Bits { by: By::Kernel, .. } => bits += 1,
                Extent::Bits {
                    by: By::Geometry, ..
                } => geo += 1,
                Extent::Word => word += 1,
                Extent::Undecided => und += 1,
                Extent::Poison => poi += 1,
            }
        }
        let _ = writeln!(
            out,
            "WEXT writes={} bits={bits} geo={geo} word={word} undecided={und} poison={poi}",
            self.writes.len()
        );
        let mut named = BTreeSet::new();
        for w in &self.writes {
            let who = match w.writer {
                Writer::Cont { row, kind } => {
                    format!("C{row}:{kind}{}", loc(sc.ca_idents.get(row as usize)))
                }
                Writer::Proc { pid, kind } => {
                    format!("P{pid}:{kind}{}", loc(sc.proc_idents.get(pid as usize)))
                }
                Writer::Body { func, task } => {
                    let f = sc
                        .func_names
                        .get(func as usize)
                        .cloned()
                        .unwrap_or_default();
                    format!("F{func}:{}:{f}", if task { "task" } else { "func" })
                }
            };
            let form = match w.form {
                Form::Continuous => "cont",
                Form::Blocking => "blk",
                Form::NonBlocking => "nba",
                Form::Force => "force",
                Form::Release => "release",
            };
            let sid = w.stmt.map_or("-".to_string(), |s| format!("s{s}"));
            let ext = match w.extent {
                Extent::Bits {
                    lo,
                    hi,
                    by: By::Kernel,
                } => format!("bits[{}:{lo}]", hi - 1),
                Extent::Bits {
                    lo,
                    hi,
                    by: By::Geometry,
                } => format!("geo[{}:{lo}]", hi - 1),
                Extent::Word => "word".to_string(),
                Extent::Undecided => "undecided".to_string(),
                Extent::Poison => "poison".to_string(),
            };
            let net = if matches!(w.extent, Extent::Poison) {
                format!("#{}", w.net)
            } else {
                named.insert(w.net);
                format!("n{}:{}", w.net, name(w.net))
            };
            let via = match w.via {
                None => "-",
                Some(Via::Direct) => "direct",
                Some(Via::TaskInline) => "task",
                Some(Via::DeferAction) => "defer",
            };
            let _ = writeln!(
                out,
                "W {who} {form} {sid} c{} {net} {ext} via={via}",
                w.chunk
            );
        }
        for n in named {
            let nv = &ir.nets[n as usize];
            let decl = match self.decl(n) {
                None => "-".to_string(),
                Some(d) => format!(
                    "{:?}/{:?}/{:?}{}",
                    d.site,
                    d.kind,
                    d.dir,
                    if d.type_param { "/tparam" } else { "" }
                ),
            };
            let _ = writeln!(
                out,
                "N n{n}:{} {:?} w={} alen={} dims={} decl={decl}",
                name(n),
                nv.kind,
                nv.width,
                nv.array_len,
                if sc.net_dims.contains_key(&n) {
                    "y"
                } else {
                    "n"
                }
            );
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn net(width: u32) -> ir::NetVar {
        ir::NetVar {
            kind: ir::NetKind::Logic,
            width,
            msb: width.saturating_sub(1),
            lsb: 0,
            signed: false,
            array_len: 1,
            dir: ir::PortDir::Internal,
            init: ir::BitPacked {
                val: vec![0],
                unk: vec![0],
            },
        }
    }

    fn whole(net: u32) -> ir::Lvalue {
        ir::Lvalue {
            chunks: vec![ir::LvalChunk {
                net,
                word: None,
                offset: None,
                width: None,
                kind: ir::SelKind::Bit,
            }],
        }
    }

    /// A chunk whose net id is not a net is `Poison` and does not index the net arena
    /// (a refused lvalue's `POISON_NET`; only an IR that latched `had_error` has one).
    #[test]
    fn a_chunk_on_no_net_is_poison() {
        let nets = vec![net(8)];
        let cas = vec![
            ir::ContAssign {
                lhs: whole(POISON_NET),
                rhs: 0,
                delay: None,
            },
            ir::ContAssign {
                lhs: whole(0),
                rhs: 0,
                delay: None,
            },
        ];
        let ids = vec![ProcIdent::default(), ProcIdent::default()];
        let v = build_view(&WextSource {
            nets: &nets,
            exprs: &[],
            consts: &[],
            cont_assigns: &cas,
            ca_idents: &ids,
            processes: &[],
            proc_idents: &[],
            stmts: &[],
            blocks: &[],
            funcs: &[],
            geom: &std::collections::HashMap::new(),
            vias: &[],
        });
        let ext: Vec<Extent> = v.writes.iter().map(|w| w.extent).collect();
        assert_eq!(
            ext,
            vec![
                Extent::Poison,
                Extent::Bits {
                    lo: 0,
                    hi: 8,
                    by: By::Kernel
                }
            ]
        );
        assert_eq!(v.writes[0].net, POISON_NET);
    }
}
