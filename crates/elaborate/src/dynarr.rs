//! dynamic arrays / queues / assoc — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

/// The element list of a queue / dynamic-array decl-init that rides the var-init
/// flush: either a `'{e0,…}` assignment pattern OR a `{e0,…}` unpacked-array
/// concatenation (IEEE §10.10 — a legal queue/dyn-array initializer too, e.g.
/// `int q[$] = {1,2,3};`). For a scalar-element target both spell the SAME element
/// list; the two forms differ only when an element is itself an unpacked array
/// (concat flattens, pattern is positional), and that case has no scalar surface so
/// `dyn_decl_init_stmts` stays loud either way — routing a `{…}` concat through the
/// same expansion as `'{…}` is therefore correct-or-loud. `{n{x}}` is a distinct
/// `Replicate` node (never `Concat`), so replication never slips in here.
pub(crate) fn dyn_pattern_elems(init: &ast::Expr) -> Option<&[ast::Expr]> {
    match &init.kind {
        ast::ExprKind::AssignPattern(parts) | ast::ExprKind::Concat { parts } => {
            Some(parts.as_slice())
        }
        _ => None,
    }
}

impl Elaborator<'_> {
    /// Expand a queue / dynamic-array `'{e0,…,eN}` decl-init into runtime statements
    /// (called from the var-init flush, in declaration order). A QUEUE pushes each
    /// element (`q.push_back(e)`); a DYNAMIC ARRAY allocates `d = new[N]` then writes
    /// each element (`d[i] = e`). Reuses the existing runtime ops — no new engine
    /// path. (Assoc / non-pattern inits do not reach here; they stay loud at the
    /// declaration.)
    pub(crate) fn dyn_decl_init_stmts(
        &self,
        name: &ast::Ident,
        kind: ir::NetKind,
        elems: &[ast::Expr],
    ) -> Vec<ast::Stmt> {
        let mut out: Vec<ast::Stmt> = Vec::new();
        let span = name.span;
        let dec = |n: usize| ast::Expr {
            kind: ast::ExprKind::IntLit {
                kind: ast::IntLitKind::Decimal,
                raw: n.to_string(),
            },
            span,
        };
        let ident_lv = |id: &ast::Ident| {
            ast::Lvalue::Ident(ast::HierPath {
                segments: vec![id.clone()],
                span: id.span,
            })
        };
        match kind {
            ir::NetKind::Queue => {
                for e in elems {
                    out.push(ast::Stmt::UserTaskCall {
                        name: ast::HierPath {
                            segments: vec![
                                name.clone(),
                                ast::Ident {
                                    name: "push_back".to_string(),
                                    span,
                                },
                            ],
                            span,
                        },
                        args: vec![e.clone()],
                        span,
                    });
                }
            }
            ir::NetKind::DynArray => {
                // d = new[N]
                out.push(ast::Stmt::Blocking {
                    lhs: ident_lv(name),
                    delay: None,
                    event: None,
                    rhs: ast::Expr {
                        kind: ast::ExprKind::New {
                            size: Box::new(dec(elems.len())),
                            src: None,
                        },
                        span,
                    },
                    span,
                });
                // d[i] = e
                for (i, e) in elems.iter().enumerate() {
                    out.push(ast::Stmt::Blocking {
                        lhs: ast::Lvalue::BitSelect {
                            base: Box::new(ident_lv(name)),
                            index: Box::new(dec(i)),
                            span,
                        },
                        delay: None,
                        event: None,
                        rhs: e.clone(),
                        span,
                    });
                }
            }
            _ => {}
        }
        out
    }

    /// R2: a read-only `input` dyn-array formal aliased to a caller DynArray net.
    pub(crate) fn dyn_subst_lookup(&self, name: &str) -> Option<u32> {
        self.dyn_subst
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, e)| *e)
    }
    // ── v5 ⑥: dynamic-storage front-end (decl/index/method lowering) ─────────

    /// The handle NetKind when `dims` declares dynamic storage.
    pub(crate) fn dyn_dim_kind(&self, dims: &[ast::Dim]) -> Option<ir::NetKind> {
        dims.iter().find_map(|d| match d {
            ast::Dim::Dyn => Some(ir::NetKind::DynArray),
            ast::Dim::Queue(_) => Some(ir::NetKind::Queue),
            ast::Dim::Assoc(ast::AssocKey::Str) => Some(ir::NetKind::AssocStr),
            ast::Dim::Assoc(_) => Some(ir::NetKind::Assoc),
            _ => None,
        })
    }

    /// `name` (single segment, current scope) as a dyn HANDLE net + its kind.
    pub(crate) fn dyn_handle(&self, name: &str) -> Option<(u32, ir::NetKind)> {
        let n = self.lookup_net_scoped(name)?;
        let k = self.nets.get(n as usize)?.kind;
        matches!(
            k,
            ir::NetKind::DynArray | ir::NetKind::Queue | ir::NetKind::Assoc | ir::NetKind::AssocStr
        )
        .then_some((n, k))
    }

    /// R2: dyn handle for a READ position — `dyn_handle` first, then a `dyn_subst`
    /// alias (a read-only `input` dyn-array formal → caller net). WRITE paths call
    /// the plain `dyn_handle` (no alias) so a write to such a formal misses `symbols`
    /// and stays loud — the read-only asymmetry is exactly this call-site choice.
    pub(crate) fn dyn_handle_read(&self, name: &str) -> Option<(u32, ir::NetKind)> {
        // The `dyn_subst` ALIAS is checked FIRST: while an R2 body is being lowered, a
        // dyn-array formal must SHADOW any outer same-named net (a module-level `int
        // b[]`/`b[$]` sharing the formal's name must NOT win — that would read the
        // wrong array). Outside an R2 body `dyn_subst` is empty, so this is exactly
        // `dyn_handle` for every other read (byte-identical).
        if let Some(net) = self.dyn_subst_lookup(name) {
            if let Some(nv) = self.nets.get(net as usize) {
                return Some((net, nv.kind));
            }
        }
        self.dyn_handle(name)
    }

    /// V2A: allocate a fresh `DynArray` temp net MIRRORING the element type of
    /// `caller_net` (an existing dyn-array handle) — width/msb/lsb/sign + 2-state
    /// membership — so the whole-handle deep-copy (`handle_copy_stmts`) accepts it (its
    /// guard louds on any element-type mismatch) and `b[i]` reads coerce X/Z→0 for a
    /// 2-state element. Holds the pass-by-value copy of an `input` dyn-array TASK formal;
    /// the synthetic name (`__dynsnap_<id>`) is never looked up (the body reads the
    /// formal, aliased to this net id via `dyn_subst`).
    pub(crate) fn alloc_dyn_snapshot(&mut self, caller_net: u32) -> u32 {
        let (width, msb, lsb, signed) = {
            let nv = &self.nets[caller_net as usize];
            (nv.width, nv.msb, nv.lsb, nv.signed)
        };
        let two_state = self.two_state_heap_handles.contains(&caller_net);
        let snap = self.nets.len() as u32;
        self.add_net(
            &format!("__dynsnap_{snap}"),
            ir::NetVar {
                kind: ir::NetKind::DynArray,
                width,
                msb,
                lsb,
                signed,
                array_len: 0, // heap handle — elements live in the engine heap
                dir: ir::PortDir::Internal,
                init: default_init(ast::NetVarKind::Reg, width.max(1)),
            },
        );
        if two_state {
            self.two_state_heap_handles.insert(snap);
        }
        snap
    }

    /// R2: the caller's DynArray NetId for a dyn-array actual — a BARE single-seg
    /// Ident resolving to a `NetKind::DynArray` net whose element width matches the
    /// formal's. `None` (→ loud) for a select / queue / assoc / non-dyn / mismatched
    /// element (correct-or-loud: the alias only shares storage of the same shape).
    pub(crate) fn dyn_array_actual_net(&mut self, a: &ast::Expr, p: &ast::TfPort) -> Option<u32> {
        let ast::ExprKind::Ident(path) = &a.kind else {
            return None;
        };
        if path.segments.len() != 1 {
            return None;
        }
        // Re-forwarding: while an R2 body is lowered, a dyn-array FORMAL name is a
        // `dyn_subst` alias to the caller's DynArray net (not a real net in `symbols`).
        // Consult it FIRST so `outer(input b[])` can pass `b` on to a nested
        // `inner(b)` (transitive read-only aliasing) — mirroring `dyn_handle_read`.
        // Outside an R2 body `dyn_subst` is empty ⇒ byte-identical to the plain lookup.
        let name = &path.segments[0].name;
        let net = self
            .dyn_subst_lookup(name)
            .or_else(|| self.lookup_net_scoped(name))?;
        let (nw, ns) = {
            let nv = self.nets.get(net as usize)?;
            if nv.kind != ir::NetKind::DynArray {
                return None;
            }
            (nv.width, nv.signed)
        };
        let kind = p.net_or_var.unwrap_or(ast::NetVarKind::Reg);
        let (fw, _, _, fs) = self.range_to_dims(kind, p.range.as_ref(), p.signed);
        // Element WIDTH *and* SIGNEDNESS must match: the alias reads the caller net's
        // own storage, so `b[i]` takes the CALLER's signedness — if that differs from
        // the formal's (`byte b[]` ← `byte unsigned a[]`), a signed element would read
        // as unsigned (0xFF → 255 vs the IEEE-correct -1). Loud on any mismatch rather
        // than silently read with the wrong sign (correct-or-loud).
        (nw == fw && ns == fs).then_some(net)
    }

    /// R2: is `s` a straight-line body (Null/Block/Blocking/Return) that never WRITES a
    /// dyn-array formal of `f`? Any control-flow statement, timing, or a dyn-formal
    /// element/whole write returns `false` → the function stays framed → loud.
    pub(crate) fn body_readonly_dyn_ok(&self, s: &ast::Stmt, f: &ast::FunctionDef) -> bool {
        use ast::Stmt as S;
        match s {
            S::Null(_) => true,
            S::Block { stmts, .. } => stmts.iter().all(|x| self.body_readonly_dyn_ok(x, f)),
            S::Return { .. } => true,
            S::Blocking {
                lhs, delay, event, ..
            } => delay.is_none() && event.is_none() && !self.lval_is_dyn_formal(lhs, f),
            _ => false,
        }
    }

    /// Is `net` (already resolved) a dyn handle? Whole-handle value surfaces
    /// (reads, whole assigns, event controls) are loud-rejected on these.
    pub(crate) fn is_dyn_handle_net(&self, net: u32) -> bool {
        matches!(
            self.nets.get(net as usize).map(|n| n.kind),
            Some(
                ir::NetKind::DynArray
                    | ir::NetKind::Queue
                    | ir::NetKind::Assoc
                    | ir::NetKind::AssocStr
            )
        )
    }

    /// Lower a dyn ELEMENT index. For a QUEUE the bare `$` substitutes
    /// `size(handle)-1` (IEEE §7.10.1: `q[$]` = the last element), scoped to
    /// THIS index by save/restore so nested selects bind `$` to their own
    /// queue. Assoc keys lower plain — the engine's signed-i64 key domain
    /// takes the expression's own width/signedness.
    pub(crate) fn lower_dyn_index(
        &mut self,
        net: u32,
        kind: ir::NetKind,
        index: &ast::Expr,
    ) -> u32 {
        if kind != ir::NetKind::Queue {
            return self.lower_expr(index);
        }
        let handle = self.push_expr(ir::Expr::Signal { net, word: None });
        let size = self.push_expr(ir::Expr::SysFunc {
            which: ir::SysFuncId::DynSize,
            args: vec![handle],
        });
        let one = self.const_u32_expr(1, 32);
        let last = self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::Sub,
            lhs: size,
            rhs: one,
        });
        let saved = self.dollar_subst.replace(last);
        let idx = self.lower_expr(index);
        self.dollar_subst = saved;
        idx
    }

    /// Read-side `handle[idx]` interception — `None` for non-handles so the
    /// caller's array/packed/scalar logic runs unchanged.
    pub(crate) fn dyn_select_read(&mut self, base: &ast::Expr, index: &ast::Expr) -> Option<u32> {
        let ast::ExprKind::Ident(p) = &base.kind else {
            return None;
        };
        if p.segments.len() != 1 {
            return None;
        }
        // R2: `dyn_handle_read` so `b[i]` on a read-only aliased input dyn-array formal
        // reads the caller's `dyn_heap[a]`.
        let (net, kind) = self.dyn_handle_read(&p.segments[0].name)?;
        let word = self.lower_dyn_index(net, kind, index);
        Some(self.push_expr(ir::Expr::Signal {
            net,
            word: Some(word),
        }))
    }

    /// The element (width, signed) of a dyn handle net, for sizing a `with`
    /// iterator's `item`.
    pub(crate) fn handle_elem_type(&self, net: u32) -> Option<(u32, bool)> {
        self.nets
            .get(net as usize)
            .map(|nv| (nv.width.max(1), nv.signed))
    }

    /// Method-call EXPRESSION on a dyn handle (`d.size()`, `a.exists(k)`…).
    /// Pops reaching HERE are NOT the direct rhs of a blocking assign (that
    /// shape is intercepted in `dyn_blocking_special`) — loud, per the engine
    /// contract (`StmtEffect::QPop` is statement-level).
    pub(crate) fn lower_dyn_method_expr(
        &mut self,
        net: u32,
        kind: ir::NetKind,
        method: &str,
        args: &[ast::Expr],
    ) -> u32 {
        use ir::NetKind as K;
        let handle = self.push_expr(ir::Expr::Signal { net, word: None });
        match (method, kind) {
            ("size", _) | ("num", K::Assoc | K::AssocStr) => {
                if !args.is_empty() {
                    self.error(MsgCode::ElabUnsupported, "size()/num() take no arguments");
                }
                let which = if method == "num" {
                    ir::SysFuncId::AssocNum
                } else {
                    ir::SysFuncId::DynSize
                };
                self.push_expr(ir::Expr::SysFunc {
                    which,
                    args: vec![handle],
                })
            }
            ("exists", K::Assoc | K::AssocStr) => {
                let Some(k) = args.first() else {
                    self.error(MsgCode::ElabUnsupported, "exists() takes the key argument");
                    return self.placeholder_expr();
                };
                let key = self.lower_expr(k);
                self.push_expr(ir::Expr::SysFunc {
                    which: ir::SysFuncId::AssocExists,
                    args: vec![handle, key],
                })
            }
            // ⓑ-breadth (v15): array reduction methods (IEEE §7.12.3). Element-
            // typed scalar result; legal on the ordered/keyed element kinds
            // (dyn array / queue / assoc) — NOT on strings (a string is a byte
            // sequence, not a numeric array; `.sum()` there is a kind error).
            (
                "sum" | "product" | "and" | "or" | "xor",
                K::DynArray | K::Queue | K::Assoc | K::AssocStr,
            ) => {
                if !args.is_empty() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "array reduction methods take no arguments (with-clause is a separate slice)",
                    );
                }
                let which = match method {
                    "sum" => ir::SysFuncId::ArrSum,
                    "product" => ir::SysFuncId::ArrProduct,
                    "and" => ir::SysFuncId::ArrAnd,
                    "or" => ir::SysFuncId::ArrOr,
                    _ => ir::SysFuncId::ArrXor,
                };
                self.push_expr(ir::Expr::SysFunc {
                    which,
                    args: vec![handle],
                })
            }
            ("pop_back" | "pop_front", K::Queue) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a queue pop is only supported as the DIRECT rhs of a blocking assignment (`x = q.pop_back();`)",
                );
                self.placeholder_expr()
            }
            // v6: the iteration methods WRITE their ref key argument — same
            // direct-rhs-only contract as the pops.
            ("first" | "next" | "last" | "prev", _) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "first/next/last/prev are only supported as the DIRECT rhs of a blocking assignment (`st = a.first(k);`)",
                );
                self.placeholder_expr()
            }
            ("push_back" | "push_front" | "delete" | "insert", _) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "statement method used in expression position",
                );
                self.placeholder_expr()
            }
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!("unknown or kind-mismatched dynamic-storage method `.{method}()`"),
                );
                self.placeholder_expr()
            }
        }
    }

    /// Method-call STATEMENT on a dyn handle (`q.push_back(v);`, `a.delete(k);`).
    pub(crate) fn lower_dyn_method_stmt(
        &mut self,
        b: &mut ProcessBuilder,
        net: u32,
        kind: ir::NetKind,
        method: &str,
        args: &[ast::Expr],
    ) {
        use ir::NetKind as K;
        let handle = self.push_expr(ir::Expr::Signal { net, word: None });
        let task = match (method, kind, args.len()) {
            ("push_back", K::Queue, 1) | ("push_front", K::Queue, 1) => {
                let v = self.lower_expr(&args[0]);
                let which = if method == "push_back" {
                    ir::SysTaskId::QPushBack
                } else {
                    ir::SysTaskId::QPushFront
                };
                ir::Stmt::SysTask {
                    which,
                    fmt: None,
                    args: vec![handle, v],
                }
            }
            ("delete", _, 0) => ir::Stmt::SysTask {
                which: ir::SysTaskId::DynDelete,
                fmt: None,
                args: vec![handle],
            },
            ("delete", K::Assoc | K::AssocStr, 1) => {
                let k = self.lower_expr(&args[0]);
                ir::Stmt::SysTask {
                    which: ir::SysTaskId::AssocDeleteKey,
                    fmt: None,
                    args: vec![handle, k],
                }
            }
            // v6: queue positional delete(i) — IEEE §7.10.2.3 (OOB/X index =
            // engine warn + skip).
            ("delete", K::Queue, 1) => {
                let i = self.lower_expr(&args[0]);
                ir::Stmt::SysTask {
                    which: ir::SysTaskId::QDeleteIdx,
                    fmt: None,
                    args: vec![handle, i],
                }
            }
            ("delete", _, 1) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "indexed delete(i) is a queue/assoc method (a dyn array only supports delete())",
                );
                return;
            }
            // v6: queue positional insert(i, v) — IEEE §7.10.2.2 (i == size
            // appends; OOB/X index = engine warn + no-op).
            ("insert", K::Queue, 2) => {
                let i = self.lower_expr(&args[0]);
                let v = self.lower_expr(&args[1]);
                ir::Stmt::SysTask {
                    which: ir::SysTaskId::QInsert,
                    fmt: None,
                    args: vec![handle, i, v],
                }
            }
            ("insert", K::Queue, _) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "insert() takes exactly (index, value)",
                );
                return;
            }
            // ⓑ-breadth (v16): ordering methods — in-place mutators on an
            // ORDERED collection (dyn array / queue). Assoc has no positional
            // order to sort (`.sort()` there is a kind error → the catch-all).
            ("sort" | "rsort" | "reverse", K::DynArray | K::Queue, 0) => {
                let which = match method {
                    "sort" => ir::SysTaskId::ArrSort,
                    "rsort" => ir::SysTaskId::ArrRsort,
                    _ => ir::SysTaskId::ArrReverse,
                };
                ir::Stmt::SysTask {
                    which,
                    fmt: None,
                    args: vec![handle],
                }
            }
            ("sort" | "rsort" | "reverse", K::DynArray | K::Queue, _) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "array ordering methods take no arguments (with-clause is a separate slice)",
                );
                return;
            }
            ("pop_back" | "pop_front", K::Queue, _) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "a queue pop result must be assigned (`x = q.pop_back();`)",
                );
                return;
            }
            ("size" | "num" | "exists", _, _) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "value-returning method used as a statement",
                );
                return;
            }
            // v6: an iteration call whose status is discarded — loud (the
            // result drives the walk; dropping it is almost surely a bug).
            ("first" | "next" | "last" | "prev", _, _) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "first/next/last/prev results must be assigned (`st = a.first(k);`)",
                );
                return;
            }
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!("unknown or kind-mismatched dynamic-storage method `.{method}()`"),
                );
                return;
            }
        };
        let sid = self.push_stmt(task);
        b.push_stmt_id(sid);
    }

    /// `d = new[n] [(src)]`, `x = q.pop_*()`, and whole-handle copy
    /// `dst = src` — the BLOCKING-assign special forms (v5 ⑥ + §7.10 copy).
    /// True ⇒ fully lowered here.
    pub(crate) fn dyn_blocking_special(
        &mut self,
        b: &mut ProcessBuilder,
        lhs: &ast::Lvalue,
        delay: Option<&ast::Delay>,
        rhs: &ast::Expr,
    ) -> bool {
        match &rhs.kind {
            // §7.5.1/§7.9/§7.10: whole-handle copy `dst = src` — VALUE
            // semantics (a DEEP copy: later writes to either side never show
            // through, iverilog-pinned for dyn/queue; assoc = hand-IEEE, no
            // oracle). Lowered as a no-op Display + `handle_copy_stmts`
            // marker (the timeformat pattern) — the engine deep-clones the
            // src heap object into dst.
            ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                let Some((src_net, src_kind)) = self.dyn_handle(&p.segments[0].name) else {
                    return false; // rhs is not a handle → the plain path
                };
                let dst = match lhs {
                    ast::Lvalue::Ident(dp) if dp.segments.len() == 1 => {
                        self.dyn_handle(&dp.segments[0].name)
                    }
                    _ => None,
                };
                let Some((dst_net, dst_kind)) = dst else {
                    // handle READ into a non-handle target (`x = q`) keeps its
                    // existing loud "no whole-value surface" on the plain path.
                    return false;
                };
                if delay.is_some() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a delayed whole-handle copy is outside the MVP",
                    );
                    return true;
                }
                if self.cur_defer.is_some() {
                    // The §16.4 push_stmt hook would capture the marker into
                    // defer_acts and try_defer would preempt the copy (the
                    // timeformat-Q1 pattern) — loud-reject the combination.
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a whole-handle copy as a deferred-assertion action is \
                         unsupported — call it as a plain statement",
                    );
                    return true;
                }
                if dst_kind != src_kind {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "whole-handle copy needs the SAME dynamic-storage kind \
                         on both sides (queue=queue, dynamic=dynamic, assoc=assoc)",
                    );
                    return true;
                }
                // §7.6 assignment compatibility: equivalent ELEMENT types only
                // (a deep clone never coerces element values, so a differing
                // width/sign/2-state elem would silently carry wrong bits).
                let elem_ty = |el: &Self, net: u32| {
                    let nv = &el.nets[net as usize];
                    (
                        nv.width,
                        nv.signed,
                        el.two_state_heap_handles.contains(&net),
                    )
                };
                if elem_ty(self, dst_net) != elem_ty(self, src_net) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "whole-handle copy needs matching element types on \
                         both sides (width/sign/state)",
                    );
                    return true;
                }
                if dst_net == src_net {
                    return true; // self-copy `q = q;` — a no-op
                }
                let sid = self.push_stmt(ir::Stmt::SysTask {
                    which: ir::SysTaskId::Display,
                    fmt: None,
                    args: Vec::new(),
                });
                self.handle_copy_stmts.insert(sid, (dst_net, src_net));
                b.push_stmt_id(sid);
                true
            }
            // §7.10.1 queue SLICE read `dst = src[a:b]` — a new queue holding
            // the in-range elements (hand-IEEE: Icarus parses this form but
            // silently ignores the bounds, so the pin is the LRM + the
            // vita-internal equivalence `q[a:b] ≡ element loop`). `$` in a
            // bound is the last index (the q[$] dollar_subst machinery).
            // Bounds are RUNTIME expressions, evaluated by the engine:
            // partial out-of-range clamps, reversed/fully-out/x-z ⇒ empty.
            ast::ExprKind::PartSelect { base, msb, lsb } => {
                let src = match &base.kind {
                    ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                        self.dyn_handle(&p.segments[0].name)
                    }
                    _ => None,
                };
                let Some((src_net, ir::NetKind::Queue)) = src else {
                    return false; // not a queue slice → the plain path louds
                };
                let dst = match lhs {
                    ast::Lvalue::Ident(dp) if dp.segments.len() == 1 => {
                        self.dyn_handle(&dp.segments[0].name)
                    }
                    _ => None,
                };
                let Some((dst_net, ir::NetKind::Queue)) = dst else {
                    return false; // slice read into a non-queue → plain-path loud
                };
                if delay.is_some() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a delayed queue-slice assignment is outside the MVP",
                    );
                    return true;
                }
                if self.cur_defer.is_some() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a queue slice as a deferred-assertion action is \
                         unsupported — call it as a plain statement",
                    );
                    return true;
                }
                let elem_ty = |el: &Self, net: u32| {
                    let nv = &el.nets[net as usize];
                    (
                        nv.width,
                        nv.signed,
                        el.two_state_heap_handles.contains(&net),
                    )
                };
                if elem_ty(self, dst_net) != elem_ty(self, src_net) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "queue slice needs matching element types on both sides \
                         (width/sign/state)",
                    );
                    return true;
                }
                let dst_h = self.push_expr(ir::Expr::Signal {
                    net: dst_net,
                    word: None,
                });
                let src_h = self.push_expr(ir::Expr::Signal {
                    net: src_net,
                    word: None,
                });
                // `$` inside a bound = src.size()-1 (the q[$] machinery).
                let a_eid = self.lower_dyn_index(src_net, ir::NetKind::Queue, msb);
                let b_eid = self.lower_dyn_index(src_net, ir::NetKind::Queue, lsb);
                let sid = self.push_stmt(ir::Stmt::SysTask {
                    which: ir::SysTaskId::Display,
                    fmt: None,
                    args: vec![dst_h, src_h, a_eid, b_eid],
                });
                self.queue_slice_stmts.insert(sid);
                b.push_stmt_id(sid);
                true
            }
            ast::ExprKind::New { size, src } => {
                // V2005 compat: a net actually named `new` → not the
                // allocation form; the plain path re-lowers it as a read.
                if self.lookup_net_scoped("new").is_some() {
                    return false;
                }
                let handle = match lhs {
                    ast::Lvalue::Ident(p) if p.segments.len() == 1 => {
                        self.dyn_handle(&p.segments[0].name)
                    }
                    _ => None,
                };
                let Some((net, ir::NetKind::DynArray)) = handle else {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "`new[n]` assigns only to a dynamic-ARRAY handle (`integer d[]; d = new[n];`)",
                    );
                    return true;
                };
                if delay.is_some() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a delayed `new[]` assignment is outside the MVP",
                    );
                    return true;
                }
                let h = self.push_expr(ir::Expr::Signal { net, word: None });
                let n_eid = self.lower_expr(size);
                let mut args = vec![h, n_eid];
                if let Some(s) = src {
                    let src_handle = match &s.kind {
                        ast::ExprKind::Ident(p) if p.segments.len() == 1 => {
                            self.dyn_handle(&p.segments[0].name)
                        }
                        _ => None,
                    };
                    let Some((src_net, ir::NetKind::DynArray)) = src_handle else {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "`new[n](src)` copy source must be a dynamic-array handle",
                        );
                        return true;
                    };
                    args.push(self.push_expr(ir::Expr::Signal {
                        net: src_net,
                        word: None,
                    }));
                }
                let sid = self.push_stmt(ir::Stmt::SysTask {
                    which: ir::SysTaskId::DynNew,
                    fmt: None,
                    args,
                });
                b.push_stmt_id(sid);
                true
            }
            // N7-REST B-CRV final: `r = obj.randomize() with {…};` — per-call inline
            // constraints. Handled before the plain Call arm.
            ast::ExprKind::RandomizeWith(rw) if rw.name.segments.len() == 2 => {
                self.try_emit_randomize(b, &rw.name, &rw.args, Some(&rw.constraints), Some(lhs))
            }
            // ⓑ-breadth (v17): `dst = src.find() with (pred)` — a queue-returning
            // locator. A reduction-with (`s = q.sum() with (…)`) is a SCALAR rhs →
            // fall through (return false) to the normal expr-rhs path.
            ast::ExprKind::ArrayMethodWith(amw) => {
                if matches!(
                    amw.method.name.as_str(),
                    "sum" | "product" | "and" | "or" | "xor"
                ) {
                    return false;
                }
                if amw.recv.segments.len() != 1 {
                    return false;
                }
                let Some((net, kind)) = self.dyn_handle(&amw.recv.segments[0].name) else {
                    return false;
                };
                let method = amw.method.name.clone();
                self.lower_locator_special(b, lhs, delay, net, kind, &method, Some(amw))
            }
            ast::ExprKind::Call { name, args } if name.segments.len() == 2 => {
                let m = name.segments[1].name.as_str();
                // N7-REST: `r = obj.randomize();` — draw the rand fields and assign
                // the success result to `lhs`.
                if m == "randomize" && self.try_emit_randomize(b, name, args, None, Some(lhs)) {
                    return true;
                }
                // A2: fixed-size unpacked array `foreach`. The parser desugars
                // `foreach (a[i])` uniformly to `__st = a.first/next(__foreach_i)`;
                // a STATIC array has no dyn handle and no `.first/.next`, so walk a
                // plain index over the first dim `[lo, lo+size)` instead. Gated to
                // the synthetic `__foreach_*` index (the user surface stays
                // assoc-only). Must precede the `dyn_handle` early-return below.
                // A multi-dim `foreach (a[i,j])` desugars to `a.first/next(idx, DIM)`
                // (2-arg, the dimension tag); a single-index `foreach (a[i])` stays the
                // 1-arg form. Both walk a plain index over the named dimension's
                // declared bounds (a static array has no `.first/.next`). Gated to the
                // synthetic `__foreach_*` index. Multi-dim on a non-fixed array is loud
                // (only fixed-size unpacked arrays carry per-dimension bounds here).
                if matches!(m, "first" | "next")
                    && delay.is_none()
                    && matches!(
                        args.first().map(|a| &a.kind),
                        Some(ast::ExprKind::Ident(p))
                            if p.segments.len() == 1
                                && p.segments[0].name.starts_with("__foreach_")
                    )
                {
                    if args.len() == 2 {
                        if let Some(arr) = self.lookup_net_scoped(&name.segments[0].name) {
                            if self.net_is_static_array(arr) {
                                let dim =
                                    self.const_eval_in_scope(&args[1]).unwrap_or(1).max(1) as u32;
                                return self
                                    .lower_fixed_foreach_step(b, lhs, arr, m, &args[0], dim);
                            }
                        }
                        self.error(
                            MsgCode::ElabUnsupported,
                            "multi-dimension foreach is supported only on fixed-size unpacked arrays",
                        );
                        return true;
                    }
                    if args.len() == 1 && self.dyn_handle_read(&name.segments[0].name).is_none() {
                        if let Some(arr) = self.lookup_net_scoped(&name.segments[0].name) {
                            if self.net_is_static_array(arr) {
                                return self.lower_fixed_foreach_step(b, lhs, arr, m, &args[0], 1);
                            }
                        }
                    }
                }
                // §4.5.174: a `foreach` READS the array — resolve via `dyn_handle_read`
                // so a read-only `input` dyn-array FORMAL (a `dyn_subst` alias, not a real
                // net) routes through the same dense dyn/queue walk a module dyn-array uses.
                // The plain `dyn_handle` misses the alias → the desugared `b.first(__i)`
                // fell through to the generic method path → E3009 (inline static-task gap,
                // §4.5.170; the frame/automatic path already worked). `dyn_subst` is empty
                // outside an inline dyn-formal body ⇒ byte-identical for every other array.
                let Some((net, kind)) = self.dyn_handle_read(&name.segments[0].name) else {
                    return false;
                };
                // v6: iteration methods — `st = h.first(k);` (ref key arg).
                if matches!(m, "first" | "next" | "last" | "prev") {
                    return self.lower_iter_special(b, lhs, delay, net, kind, m, args);
                }
                // ⓑ-breadth (v17): no-predicate locators returning a queue
                // (`r = q.min();`). The find* family needs a `with` clause and so
                // arrives via the `ArrayMethodWith` arm, never as a plain Call.
                if matches!(m, "min" | "max" | "unique" | "unique_index") && args.is_empty() {
                    return self.lower_locator_special(b, lhs, delay, net, kind, m, None);
                }
                if m != "pop_back" && m != "pop_front" {
                    return false; // size()/exists() etc. ride the normal expr path
                }
                if kind != ir::NetKind::Queue {
                    return false; // normal expr path louds the kind mismatch
                }
                if delay.is_some() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a delayed queue-pop assignment is outside the MVP",
                    );
                    return true;
                }
                if !args.is_empty() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "pop_back()/pop_front() take no arguments",
                    );
                    return true;
                }
                let handle = self.push_expr(ir::Expr::Signal { net, word: None });
                let pop = self.push_expr(ir::Expr::SysFunc {
                    which: if m == "pop_front" {
                        ir::SysFuncId::QPopFront
                    } else {
                        ir::SysFuncId::QPopBack
                    },
                    args: vec![handle],
                });
                let lv = self.lower_lvalue(lhs);
                self.check_lvalue_kind(&lv, true);
                let sid = self.push_stmt(ir::Stmt::BlockingAssign { lhs: lv, rhs: pop });
                b.push_stmt_id(sid);
                true
            }
            _ => false,
        }
    }
}
