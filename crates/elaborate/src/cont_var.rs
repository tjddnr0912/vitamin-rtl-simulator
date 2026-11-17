//! A continuous `assign` to a variable that is not `logic` (IEEE 1800-2017 §6.5, §10.3.2):
//! `reg`, `integer`, `time`, `real`, `realtime`, the 2-state integer types (`bit`, `byte`,
//! `shortint`, `int`, `longint`) and a 2-state packed struct, which sim-ir freezes as
//! `Reg`, `Integer` and `Real`.
//!
//! IEEE 1800 §6.5: "variables can be written by one continuous assignment or one port";
//! §10.3.2: "It shall be an error for a variable driven by a continuous assignment or
//! output to have an initializer in the declaration or any procedural assignment"; §6.11.2:
//! `logic` and `reg` are one type. `check_lvalue_kind` refused every such `assign` with
//! E3018 — IEEE 1364's rule, where a continuous assignment drives only a net — so a design
//! iverilog 13.0 `-g2012` and verilator 5.052 both run was refused. vita has no 1364 mode.
//!
//! The `assign` is lowered as any other (the engine treats these kinds as it treats
//! `logic`) and recorded here — a whole-array `assign` of a `reg` / 2-state struct array
//! (`cont_array.rs`) records its element rows too, so the two spellings of one drive meet
//! the same rules; [`Elaborator::check_var_cont_assign_sole_writer`] keeps it once every
//! writer exists, only where it is provably the variable's sole writer and drives all of
//! it:
//!
//! - no other writer in [`Elaborator::other_writers`]'s census — the census the whole-array
//!   `assign` (`cont_array.rs`) uses, so the two cannot disagree on what a writer is. A port
//!   connection, a procedural write (a declaration initializer's flush is one), a task or
//!   function body or copy-out, a hierarchical write, a system task's output, a clocking
//!   block drive, an `inout` connection and a `force` are writers. This is what reaches
//!   the shapes `multidriver.rs` Rule D does not: a generate-scope second `assign`, an
//!   instance output bound to the variable (`x1x0` when lifted blindly), a task-body
//!   write, a whole `assign` beside a partial procedural write;
//! - a user `assign`, not a built-in gate's output: IEEE 1800 §10.3.2 allows a variable one
//!   primitive output, but iverilog 13.0 and sv2v → iverilog refuse it ("Variable 'y'
//!   cannot be driven by a primitive or continuous assignment with non-default
//!   strength."), so only verilator, 2-state, would arbitrate its value;
//! - a right-hand side that calls no function or method;
//! - one `assign` of the whole variable, or one per element of a one-dimensional unpacked
//!   array, each a different whole element at a constant index, together covering every
//!   element (a whole-array `assign` is one per element of any array).
//!
//! Anything else stays E3018, worded for its real reason. The last two rules keep the lift
//! off two splits its `logic` twin already has (`docs/PROBE_CATALOG.md`, the §4.5.600 rows)
//! rather than add sites to them. A function body's read of a variable that is not an
//! argument re-runs the `assign` in vita and verilator, not in iverilog (`logic [7:0] w;
//! assign w = f();` with `f() = k * 2`, `k` 3 then 5: `t2 w=10` against `6`; IEEE 1800
//! §10.3.2 re-evaluates on an operand). A part select, a member, a dynamic index, a
//! multi-dimensional array's element (its flattened index carries a bounds guard that does
//! not fold) or an array not every element of which is driven leaves part of the variable
//! undriven, and that part reads differently across tools (iverilog 13.0 reads `z` where
//! vita's `logic` twin reads `x`, and reads `z` even in an `int` array element). A variable
//! an E3001 already reported (Rules A, B and D, and the overlap check), or whose
//! whole-array `assign` the sole-writer check refused (E3009), gets no E3018 on top: one
//! diagnostic per variable.
//!
//! The owner's ruling of 2026-10-06: a continuous `assign` in a file whose extension is
//! `v` (`.v` or `.V`), to a variable of a kind IEEE 1364 calls a variable (`reg`, `integer`, `time`,
//! `real`, `realtime`), is accepted with the warning [`MsgCode::ElabContAssignVar1364`] —
//! legal in IEEE 1800, an error in IEEE 1364 (iverilog `-g2005`: "Variable 'y' cannot be
//! driven by a continuous assignment/module."). It is the only file-extension-aware
//! behaviour in vita; it is not a language mode.

use super::*;
use crate::cont_array::OtherWriter;

/// One user continuous `assign` on a variable [`Elaborator::cont_var_kind`] admits.
pub(crate) struct VarContAssign {
    pub(crate) net: u32,
    /// Its row in `cont_assigns`.
    pub(crate) row: usize,
    pub(crate) span: ast::Span,
    pub(crate) prefix: String,
    /// Its right-hand side calls a function or method ([`calls_a_subroutine`]).
    pub(crate) calls: bool,
    /// A built-in gate's output, which the parser desugars to a continuous assign.
    pub(crate) gate: bool,
}

/// This module's elaborator state. Never restored.
#[derive(Default)]
pub(crate) struct ContVarState {
    /// Every recorded `assign`, in lowering order.
    pub(crate) assigns: Vec<VarContAssign>,
    /// Variables another check already reported — E3001 from `multidriver.rs` Rules A, B
    /// and D and the overlap check `check_whole_net_multidriver`, and the whole-array
    /// sole-writer E3009 (`cont_array.rs`) — so this module adds no E3018.
    pub(crate) reported: BTreeSet<u32>,
}

/// Why a recorded `assign` stays refused.
enum Refusal {
    /// The census found another writer.
    Other(OtherWriter),
    /// A written chunk names no net of this design.
    Unresolved,
    /// Two `assign`s that are not two different whole elements.
    Two,
    /// The `assign`s leave part of the variable undriven.
    Part,
    /// An element of a multi-dimensional array, driven element by element: its flattened
    /// index carries a bounds guard this check does not fold, whatever the indices are.
    MultiDim,
    /// An element index that is not a constant.
    Dynamic,
    /// The right-hand side calls a function or method.
    Call,
    /// A built-in gate's output.
    Gate,
}

/// Is `file` a Verilog file — its extension `v`, in either case (`.v`, `.V`)? The owner's
/// ruling of 2026-10-06 keys the `.v` warning on the file the `assign` is written in.
fn verilog_file(file: &str) -> bool {
    std::path::Path::new(file)
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("v"))
}

/// Does `e` call a function or a method anywhere — a user, package, hierarchical or
/// class-method call, or `randomize() with`? A system function is not one. Every child
/// is walked ([`ast::Expr::for_each_child`]), so no position can hide a call.
pub(crate) fn calls_a_subroutine(e: &ast::Expr) -> bool {
    let mut found = matches!(
        e.kind,
        ast::ExprKind::Call { .. } | ast::ExprKind::RandomizeWith(_)
    );
    e.for_each_child(|_, c| found = found || calls_a_subroutine(c));
    found
}

impl Elaborator<'_> {
    /// Is `net` a variable whose user continuous `assign` this module decides: a `Reg`,
    /// `Integer` or `Real` net that is neither a class handle (an `Integer` slot; iverilog
    /// 13.0 aborts on one, `vvp_fun_bufz: recv_object(...) not implemented`) nor a named
    /// event (refused as an lvalue, E3009)? Those two keep E3018, as `string` does.
    pub(crate) fn cont_var_kind(&self, net: u32) -> bool {
        matches!(
            self.nets.get(net as usize).map(|n| n.kind),
            Some(ir::NetKind::Reg | ir::NetKind::Integer | ir::NetKind::Real)
        ) && !self.class_handle_nets.contains(&net)
            && !self.event_nets.contains(&net)
    }

    /// Record the user `assign` (or gate output, `gate`) just pushed at `cont_assigns[row]`,
    /// once per variable it writes that [`Self::cont_var_kind`] admits; `calls` is
    /// [`calls_a_subroutine`] of its right-hand side as written.
    pub(crate) fn record_var_cont_assign(
        &mut self,
        row: usize,
        span: ast::Span,
        calls: bool,
        gate: bool,
    ) {
        let Some(ca) = self.cont_assigns.get(row) else {
            return;
        };
        let mut nets: Vec<u32> = Vec::new();
        for c in &ca.lhs.chunks {
            if !nets.contains(&c.net) && self.cont_var_kind(c.net) {
                nets.push(c.net);
            }
        }
        for net in nets {
            self.cont_var.assigns.push(VarContAssign {
                net,
                row,
                span,
                prefix: self.cur_prefix.clone(),
                calls,
                gate,
            });
        }
    }

    /// E3001 on module-scope variable `name` (`multidriver.rs`), recorded so the sole-writer
    /// check below adds no E3018 for the same variable.
    pub(crate) fn var_multidriver_error(&mut self, name: &str, span: ast::Span, msg: &str) {
        self.error_at(MsgCode::ElabMultidriver, span, msg);
        self.cont_var.reported.extend(self.lookup_net_scoped(name));
    }

    /// Keep each recorded `assign` only where the module doc's four rules hold — no other
    /// writer, no gate, no call, the whole variable driven; refuse the rest with E3018, once
    /// per variable. Runs after the last lowering and the deferred hierarchical writes, beside
    /// the whole-array check, so every writer exists.
    pub(crate) fn check_var_cont_assign_sole_writer(&mut self) {
        if self.cont_var.assigns.is_empty() {
            return;
        }
        let assigns = std::mem::take(&mut self.cont_var.assigns);
        let targets: BTreeSet<u32> = assigns.iter().map(|a| a.net).collect();
        let mut own: BTreeMap<u32, Vec<(usize, usize)>> = BTreeMap::new();
        for a in &assigns {
            own.entry(a.net).or_default().push((a.row, a.row + 1));
        }
        let (others, unknown) = self.other_writers(&targets, &own);
        // One verdict per variable, in the order its first `assign` was lowered.
        let mut seen: BTreeSet<u32> = BTreeSet::new();
        let mut refused: Vec<(&VarContAssign, Refusal)> = Vec::new();
        let mut kept: Vec<&VarContAssign> = Vec::new();
        for a in &assigns {
            if !seen.insert(a.net) || self.cont_var.reported.contains(&a.net) {
                continue;
            }
            let mine: Vec<&VarContAssign> = assigns.iter().filter(|b| b.net == a.net).collect();
            if unknown {
                refused.push((a, Refusal::Unresolved));
            } else if let Some(&w) = others.get(&a.net) {
                refused.push((a, Refusal::Other(w)));
            } else if let Some(g) = mine.iter().find(|b| b.gate) {
                refused.push((g, Refusal::Gate));
            } else if let Some(c) = mine.iter().find(|b| b.calls) {
                refused.push((c, Refusal::Call));
            } else if let Some((at, why)) = self.cont_var_cover(a.net, &mine) {
                refused.push((mine[at], why));
            } else {
                kept.extend(mine);
            }
        }
        let saved = std::mem::take(&mut self.cur_prefix);
        for (a, why) in refused {
            let name = self.cont_var_name(a.net);
            let msg = match why {
                Refusal::Other(OtherWriter::Force) => format!(
                    "variable `{name}` is driven by a continuous `assign` and also by a `force` \
                     or `release`; IEEE 1800 §10.6.2 allows that, but v1 keeps a continuous \
                     `assign` to a variable that is not `logic` only as its sole writer — \
                     declare it `logic`"
                ),
                Refusal::Other(w) => format!(
                    "variable `{name}` is driven by a continuous `assign` and also written by \
                     {}; a variable driven by a continuous assignment takes no other writer \
                     (IEEE 1800 §6.5, §10.3.2) — keep one writer",
                    w.phrase()
                ),
                Refusal::Unresolved => format!(
                    "variable `{name}` is driven by a continuous `assign`, and a hierarchical \
                     write in this design names no net vita resolved, so it may be another \
                     writer; v1 keeps a continuous `assign` to a variable that is not `logic` \
                     only as its sole writer"
                ),
                Refusal::Two => format!(
                    "variable `{name}` is driven by more than one continuous `assign`; v1 \
                     accepts that on a variable that is not `logic` only when each drives a \
                     different whole element of an unpacked array, every element once (IEEE \
                     1800 §6.5: one continuous driver per element)"
                ),
                Refusal::Gate => format!(
                    "variable `{name}` is driven by a gate output; IEEE 1800 §10.3.2 allows a \
                     variable one primitive output, but iverilog 13.0 refuses it (\"cannot be \
                     driven by a primitive or continuous assignment with non-default \
                     strength\"), so v1 keeps a gate output on a variable that is not `logic` \
                     loud — declare it a `wire`"
                ),
                Refusal::Call => format!(
                    "variable `{name}` is driven by a continuous `assign` that calls a function; \
                     v1 keeps a continuous `assign` to a variable that is not `logic` only when \
                     its right-hand side calls none — a variable the function body reads that is \
                     not an argument re-runs the `assign` here and in Verilator, not in Icarus \
                     Verilog (IEEE 1800 §10.3.2 re-evaluates on an operand)"
                ),
                Refusal::MultiDim => format!(
                    "a continuous `assign` drives an element of the multi-dimensional array \
                     `{name}`; v1 keeps a continuous `assign` to a variable that is not `logic` \
                     only when it drives the whole variable, or every element of a \
                     one-dimensional unpacked array once, at a constant index"
                ),
                Refusal::Dynamic => format!(
                    "a continuous `assign` drives an element of variable `{name}` at an index \
                     that is not a constant; v1 keeps a continuous `assign` to a variable that \
                     is not `logic` only when the `assign`s drive all of it — the whole \
                     variable, or every element of an unpacked array once, at a constant index"
                ),
                Refusal::Part => format!(
                    "a continuous `assign` drives only part of variable `{name}`; v1 keeps a \
                     continuous `assign` to a variable that is not `logic` only when the \
                     `assign`s drive all of it — the whole variable, or every element of an \
                     unpacked array once, at a constant index"
                ),
            };
            self.cur_prefix = a.prefix.clone();
            self.error_at(MsgCode::ElabLvalueKind, a.span, &msg);
        }
        // One warning per `assign` statement and variable: a whole-array `assign` is one
        // recorded row per element.
        let mut warned: BTreeSet<(u32, u32, u32)> = BTreeSet::new();
        for a in kept {
            if !warned.insert((a.net, a.span.lo, a.span.hi)) {
                continue;
            }
            if self.cont_var_1364(a.net) && verilog_file(&self.span_file_line_col(a.span).0) {
                let name = self.cont_var_name(a.net);
                self.cur_prefix = a.prefix.clone();
                self.warn_code_at(
                    MsgCode::ElabContAssignVar1364,
                    a.span,
                    &format!(
                        "continuous assignment to a variable is legal in IEEE 1800 but not in \
                         IEEE 1364 (Verilog): `{name}` is a variable, and this `assign` is in a \
                         `.v` file — declare it a `wire` for a Verilog-2005 tool"
                    ),
                );
            }
        }
        self.cur_prefix = saved;
    }

    /// `None` when the recorded `assign`s `rows` of `net` drive all of it: one `assign` of
    /// the whole variable, or one per element of an unpacked array, each a different whole
    /// element at a constant index, every element once. Otherwise the index into `rows` of
    /// the `assign` the refusal points at, and why.
    fn cont_var_cover(&self, net: u32, rows: &[&VarContAssign]) -> Option<(usize, Refusal)> {
        let len = self
            .nets
            .get(net as usize)
            .map_or(1, |n| n.array_len.max(1));
        let mut words: BTreeSet<u64> = BTreeSet::new();
        for (i, r) in rows.iter().enumerate() {
            let chunks: Vec<&ir::LvalChunk> = self
                .cont_assigns
                .get(r.row)
                .map(|ca| ca.lhs.chunks.iter().filter(|c| c.net == net).collect())
                .unwrap_or_default();
            let [c] = chunks.as_slice() else {
                return Some((i, Refusal::Part));
            };
            if c.offset.is_some() || c.width.is_some() {
                return Some((i, Refusal::Part));
            }
            match c.word {
                None if rows.len() == 1 => return None,
                None => return Some((i.max(1), Refusal::Two)),
                Some(w) => match self.const_expr_u64(w) {
                    Some(k) if k < u64::from(len) => {
                        if !words.insert(k) {
                            return Some((i, Refusal::Two));
                        }
                    }
                    Some(_) => return Some((i, Refusal::Part)),
                    None if self.net_dim_extents(net).len() >= 2 => {
                        return Some((i, Refusal::MultiDim));
                    }
                    None => return Some((i, Refusal::Dynamic)),
                },
            }
        }
        (words.len() as u64 != u64::from(len)).then_some((0, Refusal::Part))
    }

    /// Is `net` of a kind IEEE 1364 calls a variable — `reg`, `integer`, `time`, `real`,
    /// `realtime`? A net with no recorded declared kind is a non-ANSI port whose kind is
    /// 4-state (a 2-state one records its kind, `instance.rs`), so its `Reg` / `Integer` /
    /// `Real` slot is `reg` / `time`, `integer` or `real`.
    fn cont_var_1364(&self, net: u32) -> bool {
        use ast::NetVarKind as K;
        match self.intro_kind.get(&net) {
            Some(k) => matches!(k, K::Reg | K::Integer | K::Time | K::Real | K::Realtime),
            None => self.cont_var_kind(net),
        }
    }

    /// The variable's hierarchical name, as E3018 has always printed it.
    fn cont_var_name(&self, net: u32) -> String {
        self.symbols
            .iter()
            .find(|(_, &id)| id == net)
            .map(|(n, _)| n.clone())
            .unwrap_or_else(|| format!("#{net}"))
    }
}
