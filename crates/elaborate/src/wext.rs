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
//! - `Undecided`: the kernel answers `None`. That is an offset it does not read — a
//!   run-time index, a constant tree with a multiply, a cast, a conditional or a call
//!   (§5.2 Q-MDCONSTOFF's list: a packed element's `flatten_word` stride product and its
//!   bounds guard are such trees, so a packed-array element and an array-of-struct member
//!   are `Undecided` here; a genvar EXPRESSION index such as `2*g+1` stays a tree too:
//!   reach, not soundness, premise judge 1 N5) — or a chunk that writes no bit at all
//!   (wholly out of range: the kernel answers `None` for both, and so does the view).
//!   `Undecided` is not a fact: it never stands for "the whole net".
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
//! the reader's), no reader in production. The three checkers that answer parts of this
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
//! `Sidecars::write_view` (the test and dump entry) and nothing reads it in a `vita` run.
//! Built eagerly because it is cheap (Q-WREXT build r1, the view's instructions over an
//! elaboration alone, same release binary, median of 3: +0.12 % y_wext_procs, +0.27 %
//! y_wext_mix, +0.11 % y_wext_port, +0.04 % el2w; W/wrext/build/cost/probe/VIEW_COST.txt):
//! the row that reads it adds only its reader's cost.

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

/// Which bits of the net one chunk writes, as far as the finished IR decides it statically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Extent {
    /// Storage bits `[lo, hi)`, `lo < hi` (the kernel's answer).
    Bits { lo: u32, hi: u32 },
    /// An unpacked-array element: not compared bit-wise.
    Word,
    /// The kernel does not decide it (or it writes no bit).
    Undecided,
    /// The net id is not a net (an IR that latched `had_error`).
    Poison,
}

/// One write site: one chunk of one lvalue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Write {
    pub writer: Writer,
    pub form: Form,
    /// The statement id (`None` for a continuous row).
    pub stmt: Option<u32>,
    /// The chunk's index in its lvalue.
    pub chunk: u32,
    /// The chunk's net id as the IR has it (`Poison` when it is not a net).
    pub net: u32,
    pub extent: Extent,
}

/// The view: every write site, in the order the module head names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WriteView {
    pub writes: Vec<Write>,
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
}

impl Elaborator<'_> {
    /// The end phase's one build (see the module head): the view over the elaborator's
    /// own arrays, kept for `Sidecars::write_view`.
    pub(crate) fn build_write_view(&mut self) {
        self.wext = build_view(&WextSource {
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
        });
    }
}

/// The one builder.
pub(crate) fn build_view(s: &WextSource<'_>) -> WriteView {
    let ctx = ir::selfwidth::ExprCtx {
        exprs: s.exprs,
        consts: s.consts,
        nets: s.nets,
    };
    let mut writes = Vec::new();
    let mut add = |writer: Writer, form: Form, stmt: Option<u32>, lv: &ir::Lvalue| {
        for (k, c) in lv.chunks.iter().enumerate() {
            writes.push(Write {
                writer,
                form,
                stmt,
                chunk: k as u32,
                net: c.net,
                extent: chunk_extent(ctx, c),
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
    WriteView { writes }
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

/// One chunk's extent (the module head's four cases, in that order).
fn chunk_extent(ctx: ir::selfwidth::ExprCtx<'_>, c: &ir::LvalChunk) -> Extent {
    let Some(nv) = ctx.nets.get(c.net as usize) else {
        return Extent::Poison;
    };
    if c.word.is_some() {
        return Extent::Word;
    }
    match ir::const_chunk_written_bits(ctx, c, nv.width.max(1)) {
        Some((lo, hi)) => Extent::Bits { lo, hi },
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
        let (mut bits, mut word, mut und, mut poi) = (0, 0, 0, 0);
        for w in &self.writes {
            match w.extent {
                Extent::Bits { .. } => bits += 1,
                Extent::Word => word += 1,
                Extent::Undecided => und += 1,
                Extent::Poison => poi += 1,
            }
        }
        let _ = writeln!(
            out,
            "WEXT writes={} bits={bits} word={word} undecided={und} poison={poi}",
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
                Extent::Bits { lo, hi } => format!("bits[{}:{lo}]", hi - 1),
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
            let _ = writeln!(out, "W {who} {form} {sid} c{} {net} {ext}", w.chunk);
        }
        for n in named {
            let nv = &ir.nets[n as usize];
            let _ = writeln!(
                out,
                "N n{n}:{} {:?} w={} alen={} dims={}",
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
        });
        let ext: Vec<Extent> = v.writes.iter().map(|w| w.extent).collect();
        assert_eq!(ext, vec![Extent::Poison, Extent::Bits { lo: 0, hi: 8 }]);
        assert_eq!(v.writes[0].net, POISON_NET);
    }
}
