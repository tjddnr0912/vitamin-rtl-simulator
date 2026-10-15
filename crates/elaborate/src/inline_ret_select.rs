//! The inline function lane's select write of the return variable (§5.2 row 25).
//!
//! `function [3:0] decode2_4; decode2_4[3] = in[1] & in[0]; … endfunction`,
//! called from a continuous assignment, used to be E3009 ("not reducible to an
//! expression"): the straight-line fold only knew `fname = e`. A select write is
//! carried here as a MERGE into the return variable's running value — the bits
//! the select names take the new value, every other bit keeps the value the
//! body gave it so far, and a bit the body never writes is x (the declared
//! 4-state return's initial value; a 2-state return never reaches this lane, it
//! is framed by `build_frame_set`).
//!
//! Why x and not "the previous call's value" for a static function: the body is
//! straight-line and every index is an elaboration-time constant, so every call
//! writes the SAME bits; a bit one call leaves alone is a bit no call ever
//! writes. iverilog and sv2v → iverilog print those bits x (row-25 grounding
//! c06, c32; the frame lane's partial write c37 prints the same).
//!
//! Only what the merge carries exactly is accepted; everything else is refused
//! by name (`scope.named_a_reason`):
//! - the index, bounds and width must fold to constants in the self-determined
//!   domain (`const_bound_signed`, which declines a name the body binds — a
//!   formal shadowing a parameter — and an x/z); a runtime index would need the
//!   §11.5.1 "x index writes nothing" rule, which a merge cannot express;
//! - a part-select must run in the declared direction (both 4-state oracles
//!   reject a reversed one);
//! - the base must be the return variable itself (not a nested select);
//! - every name the body or its declarations resolve is declared by the body and
//!   bound before it is read (`body_binding_hazard`): the inline lane lowers the
//!   body where the CALL is, so a name that reaches the fold unbound — a parameter,
//!   module signal or package item, a local read before its first assignment or
//!   after its block, a constant in a formal's or local's range — binds in the
//!   caller's scope (grounding c49, c40; row-25 lens r1 B1/B2). That root is
//!   pre-existing for a whole write and is not widened here;
//! - the return type's range must be written in literals and fold to non-negative
//!   bounds whose width is the width the fold sizes the return with (a bound naming
//!   a constant folds where the call is too, grounding c53);
//! - in a body that writes a select of its return variable, EVERY assigned value
//!   must be repeatable (`expr_is_repeatable`): the merge names the running value
//!   once per kept slice, drops a value the select puts out of range, and the
//!   substitution fold names a local once per reference, so an effect (`$random`,
//!   a frame call) or a located read (an array element) would run a different
//!   number of times than the source says.
//!
//! An index outside the declared range writes nothing; a select partly outside
//! writes its in-range bits (§11.5.1; iverilog and sv2v, grounding c10, c46).

use super::*;

/// The return variable's declared range, folded — what a select write of it is
/// normalized against. Built in `reduce_function_body` inside the routine's
/// declaration window, so a package routine's range reads its own package.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RetSelGeom {
    pub msb: i64,
    pub lsb: i64,
    pub width: u32,
}

impl RetSelGeom {
    /// The internal bit position (0 = LSB) of declared index `idx`.
    fn internal(&self, idx: i64) -> i64 {
        if self.msb >= self.lsb {
            idx - self.lsb
        } else {
            self.lsb - idx
        }
    }

    fn descending(&self) -> bool {
        self.msb >= self.lsb
    }
}

/// The root name a select lvalue writes, peeling every select level, and whether
/// the select sits directly on that name.
fn sel_root(lv: &ast::Lvalue) -> Option<(&str, bool)> {
    let mut cur = lv;
    let mut depth = 0u32;
    loop {
        match cur {
            ast::Lvalue::BitSelect { base, .. }
            | ast::Lvalue::PartSelect { base, .. }
            | ast::Lvalue::IndexedPart { base, .. } => {
                depth += 1;
                cur = base;
            }
            ast::Lvalue::Ident(p) if depth > 0 && p.segments.len() == 1 => {
                return Some((p.segments[0].name.as_str(), depth == 1));
            }
            _ => return None,
        }
    }
}

/// Does this straight-line body write a select of the return variable `fname`?
/// Only `Block`/`Blocking` are walked: any other statement makes the body
/// non-foldable (framed, or refused by the fold), so it never reaches the merge.
pub(crate) fn body_writes_ret_select(s: &ast::Stmt, fname: &str) -> bool {
    match s {
        ast::Stmt::Block { stmts, .. } => stmts.iter().any(|st| body_writes_ret_select(st, fname)),
        ast::Stmt::Blocking { lhs, .. } => sel_root(lhs).is_some_and(|(n, _)| n == fname),
        _ => false,
    }
}

/// Why a select-writing body cannot be folded where the CALL is, or `None` when
/// every name the body or its declarations resolve is declared by the body and
/// bound before it is read.
///
/// The inline fold lowers the body at the call site: a formal is bound for the whole
/// body, a local only from its first assignment (a substitution push), block-locals
/// are flattened into one name space, and every declaration range is folded in the
/// caller's window. Any name that reaches the fold unbound therefore resolves to
/// whatever the CALLER has under it — a module or generate constant, a net, a genvar,
/// a named-block variable (row-25 lens r1: q3a, q3b, q3c, q3e, q3o, q3q, q3r, q3t,
/// q3v, q3w, q3z, q4b). This test fails closed on every such shape:
/// - a name it does not declare (a parameter, a module signal, a package item, an
///   enum label, a call, a hierarchical name, a named or type-parameter cast);
/// - a read of a local that no assignment dominates in straight-line order, in the
///   local's own block before that block exits (read-before-write, a read after the
///   block, a read of the return variable);
/// - a name declared twice (formal, local, block-local — the fold has one flat
///   name space), and a local with an initializer (a static initializer runs once,
///   not per call);
/// - a formal or local whose range, packed or unpacked dimensions or type names
///   anything (a constant, a type parameter, a class), or a cast size that does.
///
/// `decl_consts` are the names a declaration range may use: empty for a module,
/// interface or `$unit` routine (every range must be literal), and the declaring
/// package's own constants for an owned package routine, whose formal, local and
/// return ranges `reduce_function_body` folds in that package's window
/// (`decl_scope.rs`; lens r1 q3x measured it right).
pub(crate) fn body_binding_hazard(
    func: &ast::FunctionDef,
    decl_consts: &std::collections::HashSet<String>,
) -> Option<&'static str> {
    use std::collections::{HashMap, HashSet};
    const DECL: &str = "and declares a formal or local whose range or type names a constant \
         (it would fold where the call is)";
    let lit = |r: &ast::Range| {
        expr_reads_only_locals(&r.msb, decl_consts) && expr_reads_only_locals(&r.lsb, decl_consts)
    };
    let lit_dim = |d: &ast::Dim| match d {
        ast::Dim::Range(r) => lit(r),
        ast::Dim::Size(e) => expr_reads_only_locals(e, decl_consts),
        _ => false,
    };
    // Every declared name, once; formals are bound for the whole body.
    let mut owner: HashMap<String, usize> = HashMap::new(); // name -> declaring block (0 = top)
    let mut formals: HashSet<String> = HashSet::new();
    for p in &func.ports {
        if p.range.as_ref().is_some_and(|r| !lit(r))
            || !p.unpacked.iter().all(lit_dim)
            || p.shape_param.is_some()
        {
            return Some(DECL);
        }
        if owner.insert(p.name.name.clone(), 0).is_some() || p.name.name == func.name.name {
            return Some(TWICE);
        }
        formals.insert(p.name.name.clone());
    }
    if !func.ret_packed.iter().all(lit) {
        return Some(DECL);
    }
    let mut decl_ok = |d: &ast::NetVarDecl, blk: usize, owner: &mut HashMap<String, usize>| {
        if d.range.as_ref().is_some_and(|r| !lit(r))
            || !d.packed.iter().all(lit)
            || d.shape_param.is_some()
            || d.class_type.is_some()
        {
            return Some(DECL);
        }
        for n in &d.names {
            if n.init.is_some() {
                return Some("and declares a local with an initializer");
            }
            if !n.unpacked.iter().all(lit_dim) {
                return Some(DECL);
            }
            if n.name.name == func.name.name || owner.insert(n.name.name.clone(), blk).is_some() {
                return Some(TWICE);
            }
        }
        None
    };
    for d in &func.body_decls {
        if let Some(r) = decl_ok(d, 0, &mut owner) {
            return Some(r);
        }
    }
    let mut w = BindWalk {
        fname: &func.name.name,
        formals: &formals,
        owner,
        open: vec![0],
        assigned: HashSet::new(),
        next_blk: 1,
    };
    w.stmt(&func.body, &mut decl_ok)
}

const TWICE: &str = "and declares one name twice (a block-local beside a formal, a local or \
     another block's local, which the fold keeps in one name space)";
const UNBOUND: &str = "and reads a name it does not declare and bind itself before the read \
     (a parameter, a module signal, a package item, a call, a local read before it is \
     assigned or after its block, or the return variable); the inlined body would bind it \
     where the call is";

/// The straight-line walk of [`body_binding_hazard`].
struct BindWalk<'a> {
    fname: &'a str,
    formals: &'a std::collections::HashSet<String>,
    owner: std::collections::HashMap<String, usize>,
    /// The blocks whose locals are live, outermost first (0 = the function body).
    open: Vec<usize>,
    /// Locals an assignment has bound, while their block is open.
    assigned: std::collections::HashSet<String>,
    next_blk: usize,
}

impl BindWalk<'_> {
    fn stmt(
        &mut self,
        s: &ast::Stmt,
        decl_ok: &mut impl FnMut(
            &ast::NetVarDecl,
            usize,
            &mut std::collections::HashMap<String, usize>,
        ) -> Option<&'static str>,
    ) -> Option<&'static str> {
        match s {
            ast::Stmt::Null(_) => None,
            ast::Stmt::Block { decls, stmts, .. } => {
                let blk = self.next_blk;
                self.next_blk += 1;
                for d in decls {
                    if let Some(r) = decl_ok(d, blk, &mut self.owner) {
                        return Some(r);
                    }
                }
                self.open.push(blk);
                for st in stmts {
                    if let Some(r) = self.stmt(st, decl_ok) {
                        return Some(r);
                    }
                }
                self.open.pop();
                // The block's locals die with it: a later read is unbound.
                let owner = &self.owner;
                self.assigned.retain(|n| owner.get(n) != Some(&blk));
                None
            }
            ast::Stmt::Blocking { lhs, rhs, .. } => {
                if !self.expr_bound(rhs) {
                    return Some(UNBOUND);
                }
                self.lvalue(lhs)
            }
            _ => Some(UNBOUND),
        }
    }

    fn lvalue(&mut self, lv: &ast::Lvalue) -> Option<&'static str> {
        match lv {
            ast::Lvalue::Ident(p) => {
                let [seg] = p.segments.as_slice() else {
                    return Some(UNBOUND);
                };
                let n = &seg.name;
                if n == self.fname || self.formals.contains(n) {
                    return None;
                }
                match self.owner.get(n) {
                    Some(b) if self.open.contains(b) => {
                        self.assigned.insert(n.clone());
                        None
                    }
                    _ => Some(UNBOUND),
                }
            }
            ast::Lvalue::BitSelect { base, index, .. } => {
                if !self.expr_bound(index) {
                    return Some(UNBOUND);
                }
                self.sel_base(base)
            }
            ast::Lvalue::PartSelect { base, msb, lsb, .. } => {
                if !self.expr_bound(msb) || !self.expr_bound(lsb) {
                    return Some(UNBOUND);
                }
                self.sel_base(base)
            }
            ast::Lvalue::IndexedPart {
                base,
                offset,
                width,
                ..
            } => {
                if !self.expr_bound(offset) || !self.expr_bound(width) {
                    return Some(UNBOUND);
                }
                self.sel_base(base)
            }
            ast::Lvalue::Concat { .. } | ast::Lvalue::Error(_) => Some(UNBOUND),
        }
    }

    /// A select's base: the return variable, or a nested select of it (refused by
    /// the merge itself). A local's select write keeps the fold's own refusal.
    fn sel_base(&mut self, base: &ast::Lvalue) -> Option<&'static str> {
        match base {
            ast::Lvalue::Ident(p) => match p.segments.as_slice() {
                [seg] if seg.name == self.fname || self.owner.contains_key(&seg.name) => None,
                _ => Some(UNBOUND),
            },
            other => self.lvalue(other),
        }
    }

    /// Every bare name `e` reads is a formal or an assigned, live local; no other
    /// name (`expr_reads_only_locals` fails closed on the rest) and no named cast.
    fn expr_bound(&self, e: &ast::Expr) -> bool {
        let mut names: std::collections::HashSet<String> = self.formals.clone();
        names.extend(self.assigned.iter().cloned());
        expr_reads_only_locals(e, &names) && !cast_target_reads_a_name(e)
    }
}

/// `expr_reads_only_locals` reads a cast's operand, not its TARGET: a size cast
/// `K'(x)` folds `K` where the call is. A size target must name nothing at all; a
/// named or type-parameter target is a name.
fn cast_target_reads_a_name(e: &ast::Expr) -> bool {
    if let ast::ExprKind::Cast { target, .. } = &e.kind {
        match target {
            ast::CastTarget::Size(s) => {
                if !expr_reads_only_locals(s, &std::collections::HashSet::new()) {
                    return true;
                }
            }
            ast::CastTarget::Named(_) | ast::CastTarget::SigningParam { .. } => return true,
            ast::CastTarget::Prim(_) | ast::CastTarget::Signing { .. } => {}
        }
    }
    let mut hit = false;
    e.for_each_child(|_, c| hit = hit || cast_target_reads_a_name(c));
    hit
}

impl Elaborator<'_> {
    /// [`RetSelGeom`] for `func`, or `None` when its return cannot take a select
    /// write here: a `real`/`realtime`/`string` or 2-state return, a bound that
    /// does not fold or folds negative, or a folded width other than `ret_w` (the
    /// width every other write of this return is sized to). Call inside the
    /// routine's declaration window.
    pub(crate) fn inline_ret_sel_geom(
        &mut self,
        func: &ast::FunctionDef,
        ret_w: u32,
        decl_consts: &std::collections::HashSet<String>,
    ) -> Option<RetSelGeom> {
        if func.ret_string || func.ret_two_state || ret_w == 0 {
            return None;
        }
        let (msb, lsb) = match func.ret_type {
            ast::ParamType::Real | ast::ParamType::Realtime => return None,
            ast::ParamType::Integer => (31, 0),
            ast::ParamType::Time => (63, 0),
            ast::ParamType::Implicit => match &func.range {
                None => (0, 0),
                // A bound that names something folds where the CALL is, like the body
                // (`body_binding_hazard`): `function [K:0] f` called under a generate
                // block declaring its own `K` sized the return by the block's `K`
                // (grounding c53, all three oracles `00001011`, the whole-write twin
                // `00000011` — pre-existing, not widened here).
                Some(r) => {
                    if !expr_reads_only_locals(&r.msb, decl_consts)
                        || !expr_reads_only_locals(&r.lsb, decl_consts)
                    {
                        return None;
                    }
                    (
                        self.const_range_bound_fold(&r.msb)?,
                        self.const_range_bound_fold(&r.lsb)?,
                    )
                }
            },
        };
        if msb < 0 || lsb < 0 {
            return None;
        }
        let width = u32::try_from(msb.abs_diff(lsb) + 1).ok()?;
        (width == ret_w).then_some(RetSelGeom { msb, lsb, width })
    }

    /// Refuse a select write of `fname`'s return variable, naming `why`.
    pub(crate) fn refuse_ret_select(
        &mut self,
        fname: &str,
        why: &str,
        scope: &InlineScope<'_>,
    ) -> bool {
        self.error(
            MsgCode::ElabUnsupported,
            &format!(
                "function `{fname}` writes a select of its return variable {why}; an inlined \
                 function body merges a select write only where it is exact"
            ),
        );
        scope.named_a_reason.set(true);
        false
    }

    /// The `[lo, hi]` DECLARED indices a select names, or the refusal reason.
    fn ret_sel_indices(
        &self,
        lhs: &ast::Lvalue,
        geom: RetSelGeom,
    ) -> Result<(i64, i64), &'static str> {
        const NOT_CONST: &str =
            "whose index is not an elaboration-time constant (or holds x/z, or names a formal)";
        match lhs {
            ast::Lvalue::BitSelect { index, .. } => {
                let i = self.const_bound_signed(index).ok_or(NOT_CONST)?;
                Ok((i, i))
            }
            ast::Lvalue::PartSelect { msb, lsb, .. } => {
                let m = self.const_bound_signed(msb).ok_or(NOT_CONST)?;
                let l = self.const_bound_signed(lsb).ok_or(NOT_CONST)?;
                // §11.5.1: a part-select runs in the declared direction; both
                // 4-state oracles reject `f[0:3]` on a `[7:0]` return.
                if m != l && (m > l) != geom.descending() {
                    return Err("with a part-select reversed against its declared range");
                }
                Ok((m.min(l), m.max(l)))
            }
            ast::Lvalue::IndexedPart {
                offset, width, dir, ..
            } => {
                let o = self.const_bound_signed(offset).ok_or(NOT_CONST)?;
                let w = self.const_bound_signed(width).ok_or(NOT_CONST)?;
                if w < 1 {
                    return Err("with an indexed part-select width below 1");
                }
                Ok(match dir {
                    ast::PartDir::PlusColon => (o, o.saturating_add(w - 1)),
                    ast::PartDir::MinusColon => (o.saturating_sub(w - 1), o),
                })
            }
            _ => Err("in a form the merge does not carry"),
        }
    }

    /// The `fname[...] = rhs` arm of `fold_straight_line`. `None` = `lhs` is not a
    /// select of the return variable (the caller keeps its own verdict); `Some(ok)`
    /// = handled, `ok == false` after a named refusal.
    pub(crate) fn fold_ret_select_write(
        &mut self,
        lhs: &ast::Lvalue,
        rhs: &ast::Expr,
        fname: &str,
        ret_signed: bool,
        scope: &InlineScope<'_>,
        ret: &mut Option<u32>,
    ) -> Option<bool> {
        let (root, direct) = sel_root(lhs)?;
        if root != fname {
            return None;
        }
        if !direct {
            return Some(self.refuse_ret_select(fname, "through a nested select", scope));
        }
        let Some(geom) = scope.ret_sel else {
            return Some(self.refuse_ret_select(
                fname,
                "whose declared type has no literal, non-negative bit range here \
                 (a real, string or 2-state return, or a range that names a constant)",
                scope,
            ));
        };
        let (a, b) = match self.ret_sel_indices(lhs, geom) {
            Ok(r) => r,
            Err(why) => return Some(self.refuse_ret_select(fname, why, scope)),
        };
        let (ia, ib) = (geom.internal(a), geom.internal(b));
        let (lo, hi) = (ia.min(ib), ia.max(ib));
        let Some(n) = u32::try_from(hi - lo + 1)
            .ok()
            .filter(|&n| u64::from(n) <= MAX_NET_WIDTH)
        else {
            return Some(self.refuse_ret_select(fname, "wider than a net may be", scope));
        };
        // The value, sized to the select (§11.6.1: the select is the assignment's
        // unsigned target), exactly as the whole-write arm sizes it to a target.
        let v0 = self.lower_inline_assign_rhs(rhs, n, true);
        if self.inline_stream_store_declines(rhs, v0, n) {
            self.error_stream_inline_store();
            scope.named_a_reason.set(true);
            return Some(false);
        }
        let v = if self.expr_is_real(v0) {
            self.real_to_int_store(v0, n, false)
        } else if self.cast_operand_is_real(rhs, v0) {
            return Some(self.refuse_ret_select(
                fname,
                "with a real value the inline fold cannot convert",
                scope,
            ));
        } else if self.trusted_self_width(v0).is_none() {
            return Some(self.refuse_ret_select(
                fname,
                "with a value whose width is not known here",
                scope,
            ));
        } else {
            self.resize_inline_assign(v0, n, false)
        };
        if !self.expr_is_repeatable(v) {
            return Some(self.refuse_value_not_repeatable(fname, scope));
        }
        let w = geom.width;
        let base = match *ret {
            Some(r) => r,
            None => {
                let cid = self.intern_const(ir::ConstVal {
                    width: w,
                    signed: false,
                    repr: ir::ConstRepr::Numeric,
                    bits: default_init(ast::NetVarKind::Reg, w),
                });
                self.push_expr(ir::Expr::Const { val: cid })
            }
        };
        if self.trusted_self_width(base) != Some(w) {
            return Some(self.refuse_ret_select(
                fname,
                "after a write whose width is not known here",
                scope,
            ));
        }
        // §11.5.1: only the in-range bits are written.
        let (c_lo, c_hi) = (lo.max(0), hi.min(i64::from(w) - 1));
        if c_lo > c_hi {
            *ret = Some(base);
            return Some(true);
        }
        let (c_lo, c_hi) = (c_lo as u32, c_hi as u32);
        let kept = c_hi - c_lo + 1;
        let val = if kept == n {
            v
        } else {
            self.select_bits(v, (i64::from(c_lo) - lo) as u32, kept)
        };
        let mut parts = Vec::with_capacity(3);
        if c_hi + 1 < w {
            parts.push(self.select_bits(base, c_hi + 1, w - 1 - c_hi));
        }
        parts.push(val);
        if c_lo > 0 {
            parts.push(self.select_bits(base, 0, c_lo));
        }
        let merged = if parts.len() == 1 {
            val
        } else {
            self.push_expr(ir::Expr::Concat { parts })
        };
        let which = if ret_signed {
            ir::SysFuncId::Signed
        } else {
            ir::SysFuncId::Unsigned
        };
        *ret = Some(self.push_expr(ir::Expr::SysFunc {
            which,
            args: vec![merged],
        }));
        Some(true)
    }

    /// `Select(e, off, n)` — `n` unsigned bits of `e` from internal bit `off`.
    fn select_bits(&mut self, e: u32, off: u32, n: u32) -> u32 {
        let off = self.const_u32_expr(off, 32);
        let wid = self.const_u32_expr(n, 32);
        self.push_expr(ir::Expr::Select {
            base: e,
            offset: off,
            width: wid,
            kind: ir::SelKind::PartConst,
        })
    }

    /// The refusal for a non-repeatable value in a body that writes a select of
    /// its return variable (see the module header).
    pub(crate) fn refuse_value_not_repeatable(
        &mut self,
        fname: &str,
        scope: &InlineScope<'_>,
    ) -> bool {
        self.refuse_ret_select(
            fname,
            "and assigns a value that has an effect or reads an array element (a call, \
             `$random`, `mem[i]`), which the merge could evaluate a different number of \
             times than the body does",
            scope,
        )
    }
}
