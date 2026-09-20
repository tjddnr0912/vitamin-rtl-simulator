//! A SELECT into an element of dynamic storage whose element type carries more
//! than one packed dimension.
//!
//! `typedef logic [1:0][3:0] t_ex; t_ex q [$];` stores each element as one flat
//! 8-bit value (the handle net's `width` is the product of the packed extents,
//! and a heap element has no static extent table of its own). A second index —
//! `q[0][1]` — therefore lowers through the ordinary vector path and reads BIT 1
//! of that flat element, where the language names the outer packed ELEMENT
//! (verilator: `a`; vita: `0`). iverilog refuses the two-index form outright
//! ("the number of indices (2) is greater than the number of dimensions (1)"),
//! so it is not an oracle here.
//!
//! The value is unrecoverable without per-element extents, so the select is
//! LOUD. Only the select: a whole-element read or write (`q[0]`, `d[i] = v`,
//! `q.push_back(v)`, `q.size()`) is already correct and is untouched, and a
//! ONE-dimensional element (`logic [7:0] q1 [$]`, where `q1[0][1]` is a genuine
//! bit-select and vita already agrees with verilator) is untouched as well —
//! the handle is recorded in `dyn_md_elem` only when the declaration carries an
//! inner packed-dim list (`netdecl.rs`).
use super::*;

impl Elaborator<'_> {
    /// Is `base` — the BASE of an enclosing select — an element select
    /// (`q[i]`) of a dynamic-storage handle whose element type has more than
    /// one packed dimension? Reports and returns `true` when it is, so the
    /// caller returns a placeholder instead of lowering a flat bit-select.
    ///
    /// The handle is resolved with `dyn_handle_read`, the SAME resolver the
    /// element-read lowering (`dyn_select_read`) uses, so the refusal and the
    /// lowering can never disagree about which name is a handle.
    pub(crate) fn reject_dyn_md_elem_select(&mut self, base: &ast::Expr) -> bool {
        let ast::ExprKind::BitSelect { base: inner, .. } = &base.kind else {
            return false;
        };
        let ast::ExprKind::Ident(path) = &inner.kind else {
            return false;
        };
        if path.segments.len() != 1 {
            return false;
        }
        let Some((net, _)) = self.dyn_handle_read(&path.segments[0].name, path.span) else {
            return false;
        };
        if !self.dyn_md_elem.contains(&net) {
            return false;
        }
        self.error(
            MsgCode::ElabUnsupported,
            "a select into an element of a dynamic array / queue / associative array \
             whose element type has more than one packed dimension is unsupported in \
             v1 (read or write the whole element)",
        );
        true
    }
}
