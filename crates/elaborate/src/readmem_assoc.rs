//! §5.2 row 27: `$readmemb` / `$readmemh` into an associative array — elaborate's
//! half. The engine's half is `sim-engine` `builtins::readmem_assoc`.
//!
//! The memory argument naming a WHOLE associative array (local, or a cross-instance
//! name once it resolves) lowers to the marker
//! `SysFunc { ReadmemAssocTarget, [Signal { net, word: None }, Const k] }`, where
//! `k`'s width and sign are the key type the loader needs and the IR has no other
//! field for. The engine loads an associative array only from that shape, and no
//! element select lowers to it, so `$readmemh(f, mem[5])` keeps the fixed-memory
//! loader's "not a memory" warning (round-1 lens F1: a constant-index element
//! select is `Signal { net, word: Some(Const) }`, the shape round 1 used).
//!
//! Three targets are refused by name (E3009), each where the loader has no settled
//! reading:
//!
//! * a `[string]` key: a `$readmem` address is integral (verilator 5.052:
//!   `$readmemh address/key must be integral`);
//! * an element type that is not integral (a class handle, `string`, `real`, an
//!   unpacked struct, …): a `$readmem` word is an integral value (verilator 5.052:
//!   `$readmemh array values must be integral`). Positive set: only an array
//!   declared with an integral element kind is recorded (`assoc_integral_elem`);
//! * an array that a `wait` condition reads ([`Elaborator::check_readmem_assoc_waits`]):
//!   a `wait` over a heap container does not wake when the container's elements are
//!   written (ROADMAP §2, on queues, dynamic and associative arrays alike), so the
//!   load would leave that `wait` asleep where verilator wakes it.

use super::*;

/// One `$readmem*` whose memory argument is an associative array's marker, kept
/// for the `wait` check that runs once every process and function body exists.
#[derive(Debug, Clone)]
pub(crate) struct ReadmemAssocSite {
    pub(crate) net: u32,
    pub(crate) span: Option<ast::Span>,
    pub(crate) prefix: String,
    pub(crate) name: String,
}

/// An associative-array element kind a `$readmem*` word can be stored into: the
/// integral kinds. A class handle, `string`, `real`, `event` or virtual interface is
/// not; a net kind cannot be dynamic storage at all.
pub(crate) fn assoc_elem_kind_is_integral(kind: ast::NetVarKind) -> bool {
    use ast::NetVarKind::*;
    matches!(
        kind,
        Reg | Logic | Integer | Time | Bit | Byte | Shortint | Int | Longint
    )
}

impl Elaborator<'_> {
    /// The marker for `$readmem*`'s memory argument naming the associative array
    /// `net` (spelled `name` in the source), or `None` when the target is refused
    /// (reported here).
    pub(crate) fn readmem_assoc_marker(&mut self, net: u32, name: &str) -> Option<ir::Expr> {
        match self.nets.get(net as usize).map(|n| n.kind) {
            Some(ir::NetKind::Assoc) => {}
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "`$readmemb`/`$readmemh` into an associative array with a `string` key: \
                     a `$readmem` address is integral",
                );
                return None;
            }
        }
        if !self.assoc_integral_elem.contains(&net) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "`$readmemb`/`$readmemh` into the associative array `{name}`, whose \
                     element type is not integral: a `$readmem` word is an integral value"
                ),
            );
            return None;
        }
        // A keyword or typed integral key carries its folded shape; `[time]` (no
        // folded shape, ROADMAP §2) is 64-bit unsigned.
        let (width, signed) = self
            .assoc_key_shapes
            .get(&net)
            .map_or((64, false), |k| (k.width, k.signed));
        let cid = self.intern_const(ir::ConstVal {
            width,
            signed,
            repr: ir::ConstRepr::Numeric,
            bits: ir::BitPacked {
                val: vec![0],
                unk: vec![0],
            },
        });
        let key = self.push_expr(ir::Expr::Const { val: cid });
        let sig = self.push_expr(ir::Expr::Signal { net, word: None });
        self.readmem_assoc_sites.push(ReadmemAssocSite {
            net,
            span: self.cur_span,
            prefix: self.cur_prefix.clone(),
            name: name.to_string(),
        });
        Some(ir::Expr::SysFunc {
            which: ir::SysFuncId::ReadmemAssocTarget,
            args: vec![sig, key],
        })
    }

    /// The local arm: `Some(eid)` when the `$readmem*` memory argument `e` names an
    /// associative array in scope (the marker, or a placeholder when refused),
    /// `None` for every other target, which the caller lowers as before.
    pub(crate) fn readmem_assoc_target(&mut self, e: &ast::Expr) -> Option<u32> {
        if let ast::ExprKind::Paren { inner } = &e.kind {
            return self.readmem_assoc_target(inner);
        }
        let net = self.whole_name_net(e)?;
        if !matches!(
            self.nets.get(net as usize).map(|n| n.kind),
            Some(ir::NetKind::Assoc | ir::NetKind::AssocStr)
        ) {
            return None;
        }
        let name = match &e.kind {
            ast::ExprKind::Ident(p) => p
                .segments
                .iter()
                .map(|s| s.name.as_str())
                .collect::<Vec<_>>()
                .join("."),
            _ => String::new(),
        };
        match self.readmem_assoc_marker(net, &name) {
            Some(m) => Some(self.push_expr(m)),
            None => Some(self.placeholder_expr()),
        }
    }

    /// Every net a `wait` condition anywhere in the design may read: directly, through
    /// a method or an element read (the handle's `Signal`), or inside a function the
    /// condition calls (every function reachable from it; a virtual call reaches every
    /// override in its slot). `None` when a callee cannot be found — the caller then
    /// treats every array as read.
    fn wait_read_nets(&self) -> Option<std::collections::BTreeSet<u32>> {
        let mut reads = std::collections::BTreeSet::new();
        let mut calls: Vec<u32> = Vec::new();
        let blocks = self
            .processes
            .iter()
            .flat_map(|p| p.body.iter())
            .chain(self.func_blocks.iter());
        for bb in blocks {
            if let ir::Terminator::Wait {
                cond: ir::WaitCause::Expr { expr },
                ..
            } = &bb.term
            {
                self.collect_expr_reads_calls(*expr, &mut reads, &mut calls);
            }
        }
        let mut visited = std::collections::BTreeSet::new();
        while let Some(fid) = calls.pop() {
            if !visited.insert(fid) {
                continue;
            }
            // A virtual call names its static method; the run may dispatch to any
            // override in the same slot.
            for (vslot, static_fid) in self.class_calls.values() {
                if *static_fid != fid {
                    continue;
                }
                let Some(slot) = vslot else { continue };
                for row in &self.class_vtable {
                    match row.get(*slot as usize) {
                        // `finalize_vtables` pads a class that has no method in this
                        // slot (an unrelated class, or a base below the class that
                        // introduced it) with `u32::MAX`: an absent method, which no
                        // call dispatches to, not an unknown callee.
                        Some(&u32::MAX) => {}
                        Some(&f) => calls.push(f),
                        None => return None,
                    }
                }
            }
            let f = self.funcs.get(fid as usize)?;
            let mut bbs = vec![f.entry];
            let mut seen = std::collections::BTreeSet::new();
            while let Some(b) = bbs.pop() {
                if !seen.insert(b) {
                    continue;
                }
                let bb = self.func_blocks.get(b as usize)?;
                self.collect_block_reads(bb, &mut reads, &mut calls);
                match &bb.term {
                    ir::Terminator::Goto { target } => bbs.push(*target),
                    ir::Terminator::Branch {
                        then_bb, else_bb, ..
                    } => {
                        bbs.push(*then_bb);
                        bbs.push(*else_bb);
                    }
                    ir::Terminator::Delay { amount, resume, .. } => {
                        self.collect_expr_reads_calls(*amount, &mut reads, &mut calls);
                        bbs.push(*resume);
                    }
                    ir::Terminator::Wait { resume, .. } => bbs.push(*resume),
                    ir::Terminator::Fork {
                        children,
                        join,
                        resume_bb,
                    } => {
                        bbs.extend(children.iter().copied());
                        bbs.push(*join);
                        bbs.push(*resume_bb);
                    }
                    ir::Terminator::Call { target, ret_bb } => {
                        calls.push(*target);
                        bbs.push(*ret_bb);
                    }
                    ir::Terminator::Return => {}
                }
            }
        }
        Some(reads)
    }

    /// Refuse every `$readmem*` into an associative array that a `wait` condition
    /// reads (E3009, at the `$readmem*`). The engine wakes a `wait` on a net change,
    /// and an element write to a heap container (`mem[k] = v`, `q.push_back(v)`, and
    /// this loader) is not one: `wait (mem.num() == 2)` never wakes (ROADMAP §2;
    /// verilator wakes it). Run once every process, function body and hierarchical
    /// name exists.
    pub(crate) fn check_readmem_assoc_waits(&mut self) {
        if self.readmem_assoc_sites.is_empty() {
            return;
        }
        let read = self.wait_read_nets();
        let sites = std::mem::take(&mut self.readmem_assoc_sites);
        let (span, prefix) = (self.cur_span, self.cur_prefix.clone());
        for s in &sites {
            let hit = read.as_ref().is_none_or(|r| r.contains(&s.net));
            if !hit {
                continue;
            }
            self.cur_span = s.span;
            self.cur_prefix = s.prefix.clone();
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "`$readmemb`/`$readmemh` into the associative array `{}`, which a `wait` \
                     condition reads: a `wait` over an associative array does not wake when \
                     its elements are written, so the load would leave that `wait` asleep",
                    s.name
                ),
            );
        }
        self.cur_span = span;
        self.cur_prefix = prefix;
    }
}
