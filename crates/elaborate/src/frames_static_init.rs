//! §6.21/§13.4.1 once-only STATIC frame-local initializers — split out of
//! `frames_body.rs` at the 1000-line cap. Mechanical move: every item below is verbatim
//! from that file, in the same `impl Elaborator<'_>` block, and nothing else changed.
//! No `#[derive(SchemaHash)]` type moves with it (these are functions only), so the
//! `module_path!()`-keyed canonical hashes are untouched.

use super::*;

impl Elaborator<'_> {
    /// The `$blk$<lo>` segment the LOWERING wraps a `begin … end` body in, or `None`.
    ///
    /// The ONE spelling of that rule. `Stmt::Block` calls it to build the scope it lowers
    /// the body under, and [`Self::block_wrap_prefix`] calls it to reproduce that scope
    /// when a pass has to resolve a name the way the lowering will.
    ///
    /// ⚠️ NOT `block_local_scope_prefix`. That one asks `block_local_scope_seg`, which is
    /// per-DECLARATION and omits the segment for a declaration the admission rules did not
    /// select; the wrap is per-BLOCK and applies to every declaration in it, selected or
    /// not. Resolving admission under the first and emitting under the second is how an
    /// unselected declarator's initializer came to be claimed by neither emitter.
    pub(crate) fn block_wrap_seg(&self, span_lo: u32) -> Option<String> {
        self.scoped_block_locals
            .contains_key(&span_lo)
            .then(|| format!("$blk${span_lo}"))
    }

    /// The whole prefix the lowering is under inside the innermost block of `chain` — one
    /// [`Self::block_wrap_seg`] per enclosing block that has one, joined outermost-first.
    pub(crate) fn block_wrap_prefix(&self, chain: &[ast::Span]) -> Option<String> {
        let segs: Vec<String> = chain
            .iter()
            .filter_map(|sp| self.block_wrap_seg(sp.lo))
            .collect();
        (!segs.is_empty()).then(|| segs.join("."))
    }

    /// Does this declarator's initializer belong to the ONCE-only prologue rather than
    /// the per-activation emission? The ONE predicate both emission points ask, so the
    /// prologue and `emit_frame_local_inits` cannot disagree about which initializer is
    /// emitted where (one that neither claimed would be dropped outright).
    pub(crate) fn frame_static_init_once(&self, d: &ast::NetVarDecl, init: &ast::Expr) -> bool {
        !d.lifetime.unwrap_or(self.frame_body_auto) && self.frame_static_init_t0_safe(init)
    }

    /// Emit this frame's STATIC declaration initializers ONCE, at the top of the body,
    /// behind the frame's own `$sinit$` flag (`reserve_frame_static_guard`).
    ///
    /// The frame twin of `hoist_inline_task_locals`' `first_call` gate — which is why the
    /// INLINE route was already correct for this shape and the frame route was not. The
    /// flag and every static local live in the frame's persistent static slab, so the
    /// initializers run on the first activation and never again:
    ///
    /// ```text
    ///   if ($sinit === 1) goto body; else { $sinit = 1; <static inits>; goto body; }
    /// ```
    ///
    /// Both the frame's own `body_decls` and every NESTED block's decls are hoisted here,
    /// each lowered under the `$blk$<lo>` segment its own block uses, so a static
    /// loop-body local (`for (…) begin int z = 100; …`) initializes once across every
    /// iteration and every call — measured on iverilog 13 and verilator as
    /// `100 101 102 103 104 105` over two three-iteration calls.
    ///
    /// A frame with no admitted static initializer reserved no flag and returns here
    /// before touching the builder (byte-identical IR).
    pub(crate) fn emit_frame_static_prologue(
        &mut self,
        b: &mut ProcessBuilder,
        body_decls: &[ast::NetVarDecl],
        body: &ast::Stmt,
        ports: &[ast::TfPort],
        fid: u32,
    ) {
        let owner = self.func_metas[fid as usize].base_net;
        let Some(&guard) = self.frame_static_guard.get(&owner) else {
            return;
        };
        // (scope segment, declaration) for every candidate, in declaration order:
        // the frame's own decls first, then the nested blocks' in source order.
        let mut cands: Vec<(Option<String>, ast::NetVarDecl)> =
            body_decls.iter().map(|d| (None, d.clone())).collect();
        let mut nested = Vec::new();
        crate::block_local::collect_block_local_decls_spanned(body, &mut Vec::new(), &mut nested);
        for (chain, d) in &nested {
            // The prefix the `Stmt::Block` lowering will be under, NOT the per-declaration
            // `block_local_scope_prefix` — see `block_wrap_seg`.
            let seg = self.block_wrap_prefix(chain);
            cands.push((seg, d.clone()));
        }
        // The NON-HOISTABLE set, before anything is admitted: this frame's FORMALS, then
        // every local whose own initializer is declined, to a fixpoint (a chain
        // `int c = k; int d = c + 1;` declines BOTH — `d` reading `c` at first activation
        // would be a third answer just as `c` reading `k` is). Only ever grows, bounded by
        // the declaration count, so the loop terminates.
        let formals: std::collections::BTreeSet<String> = ports
            .iter()
            .filter_map(|p| self.walk_scopes_key(&p.name.name, |k| self.symbols.contains_key(k)))
            .collect();
        let mut nonh = formals.clone();
        loop {
            let before = nonh.len();
            self.frame_nonhoistable.clone_from(&nonh);
            for (seg, d) in &cands {
                let d = d.clone();
                let found = match seg {
                    Some(seg) => self.with_scope(seg, |s| s.declined_local_keys(&d)),
                    None => self.declined_local_keys(&d),
                };
                nonh.extend(found);
            }
            if nonh.len() == before {
                break;
            }
        }
        self.frame_nonhoistable = nonh;
        // PER-DECLARATOR, with ONE frame-wide escape hatch.
        //
        // A declined sibling does not by itself cost an admitted declarator its once-only
        // initializer: measured on `function int fmix(); int a = 15; int b = outside;
        // a=a+1; b=b+1;` called twice, BOTH oracles retain `a` (iverilog `16001 17002`,
        // verilator `16051 17052` — they differ only on `b`'s initial value, never on
        // whether `a` retains), so declining the whole frame for `b`'s sake left `a` at
        // `16051 16051`, which is neither oracle.
        //
        // The shape that takes the whole frame is a DECLINED initializer that READS an
        // ADMITTED local of this frame. That local is hoisted, so it retains and the body
        // mutates it; the declined initializer then re-runs per activation and reads the
        // MUTATED value. Both halves of that were measured against a build with this return
        // removed:
        //
        //  * `function int f(int k); int a = 5; int b = a + k;` called f(1), f(2) — the read
        //    is declined because of the FORMAL. Without the return: `7 9`. iverilog 13 says
        //    `6 7` and verilator 5.052 refuses the design outright
        //    (`%Error-UNSUPPORTED: Static variable initializer`), so `7 9` is an answer no
        //    oracle gives. Keeping the pre-slice emission whole gives `7 8` — also neither,
        //    but it is the pre-slice value, so the slice moves nothing rather than inventing.
        //  * `function int f; int a = 15; int b = a + outside;` — the read is declined
        //    because of a MODULE NET. Without the return: `16066 17067`, which is verilator
        //    5.052 VERBATIM (iverilog `16016 17017`). Here the return costs a real oracle
        //    match, so it is a CONSERVATIVE choice, not a correctness one.
        //
        // One rule covers both because the two cases differ only in WHY the sibling was
        // declined, and that reason is exactly the open oracle-split question. Narrowing the
        // hatch to the formal case is a separate decision that needs that split resolved.
        let mut admitted_keys: std::collections::BTreeSet<String> = Default::default();
        for (seg, d) in &cands {
            let d = d.clone();
            let found = match seg {
                Some(seg) => self.with_scope(seg, |s| s.admitted_local_keys(&d)),
                None => self.admitted_local_keys(&d),
            };
            admitted_keys.extend(found);
        }
        let mut shadowed = false;
        for (seg, d) in &cands {
            let d = d.clone();
            shadowed |= match seg {
                Some(seg) => {
                    let seg = seg.clone();
                    let keys = admitted_keys.clone();
                    self.with_scope(&seg, |s| s.declined_init_reads_admitted(&d, &keys))
                }
                None => self.declined_init_reads_admitted(&d, &admitted_keys),
            };
        }
        if shadowed {
            self.frame_nonhoistable.clear();
            return;
        }
        // Admission is decided BEFORE the builder is touched: `reserve_frame_static_guard`
        // admits on "static declarator with an initializer", this emitter additionally on
        // `frame_static_init_t0_safe`, so a reserved flag does not imply an emission (an
        // `int a = f2();` is reserved for and then declined here, staying loud). A frame
        // with nothing admitted must add no branch and no flag write.
        let admitted: Vec<bool> = cands
            .iter()
            .map(|(seg, d)| match seg {
                Some(seg) => {
                    let d = d.clone();
                    self.with_scope(seg, |s| s.decl_has_static_once(&d))
                }
                None => self.decl_has_static_once(d),
            })
            .collect();
        if !admitted.iter().any(|a| *a) {
            return;
        }
        let init_bb = b.new_block();
        let merge = b.new_block();
        let flag = self.push_expr(ir::Expr::Signal {
            net: guard,
            word: None,
        });
        let one = self.const_s32_expr(1);
        let done = self.push_expr(ir::Expr::Binary {
            op: ir::BinOp::CaseEq,
            lhs: flag,
            rhs: one,
        });
        b.end_block_with(ir::Terminator::Branch {
            cond: done,
            then_bb: merge.raw(),
            else_bb: init_bb.raw(),
        });
        b.start_block(init_bb);
        // Set the flag FIRST: a static initializer is emitted through the ordinary
        // statement lowering, which may end the open block (a nested branch), and the
        // flag write has to be on every path out of this prologue.
        let set = self.push_stmt(ir::Stmt::BlockingAssign {
            lhs: crate::lvalue::whole_net_lvalue(guard),
            rhs: one,
        });
        b.push_stmt_id(set);
        for (seg, d) in cands {
            match seg {
                Some(seg) => {
                    self.with_scope(&seg, |s| s.emit_static_decl_inits(b, &d));
                }
                None => self.emit_static_decl_inits(b, &d),
            }
        }
        b.goto(merge);
        b.start_block(merge);
    }

    /// The resolved keys of this declaration's STATIC declarators whose initializer is
    /// DECLINED — they keep the per-activation emission, so anything reading them cannot
    /// be hoisted either. Resolved under the CURRENT prefix.
    fn declined_local_keys(&self, d: &ast::NetVarDecl) -> Vec<String> {
        if d.lifetime.unwrap_or(self.frame_body_auto) {
            return Vec::new();
        }
        d.names
            .iter()
            .filter(|decl| {
                decl.init
                    .as_ref()
                    .is_some_and(|init| !self.frame_static_init_t0_safe(init))
            })
            .filter_map(|decl| {
                self.walk_scopes_key(&decl.name.name, |k| self.symbols.contains_key(k))
            })
            .collect()
    }

    /// The resolved keys of this declaration's declarators that ARE hoisted, under the
    /// CURRENT prefix. The mirror of [`Self::declined_local_keys`].
    fn admitted_local_keys(&self, d: &ast::NetVarDecl) -> Vec<String> {
        d.names
            .iter()
            .filter(|decl| {
                decl.init
                    .as_ref()
                    .is_some_and(|init| self.frame_static_init_once(d, init))
            })
            .filter_map(|decl| {
                self.walk_scopes_key(&decl.name.name, |k| self.symbols.contains_key(k))
            })
            .collect()
    }

    /// Does a DECLINED initializer in this declaration read one of `admitted`?
    ///
    /// FAIL-CLOSED: an expression shape this walk cannot decompose answers `true`, because
    /// a reference it cannot see is exactly the reference that produces the third answer.
    fn declined_init_reads_admitted(
        &self,
        d: &ast::NetVarDecl,
        admitted: &std::collections::BTreeSet<String>,
    ) -> bool {
        d.names
            .iter()
            .filter_map(|decl| decl.init.as_ref())
            .filter(|init| !self.frame_static_init_once(d, init))
            .any(|init| self.expr_reads_admitted(init, admitted))
    }

    /// [`Self::declined_init_reads_admitted`]'s expression walk. Every shape that can hold a
    /// sub-expression recurses; a name is tested by RESOLVED key, so a block-local that
    /// merely shares a spelling with an admitted one elsewhere does not match. Any other
    /// shape returns `true` (fail closed).
    fn expr_reads_admitted(
        &self,
        e: &ast::Expr,
        admitted: &std::collections::BTreeSet<String>,
    ) -> bool {
        use ast::ExprKind as K;
        let any = |v: &[ast::Expr]| v.iter().any(|x| self.expr_reads_admitted(x, admitted));
        match &e.kind {
            K::IntLit { .. } | K::RealLit { .. } | K::StrLit { .. } | K::PkgScoped { .. } => false,
            K::Ident(p) => {
                p.segments.len() == 1 && {
                    self.walk_scopes_key(&p.segments[0].name, |k| self.symbols.contains_key(k))
                        .is_some_and(|k| admitted.contains(&k))
                }
            }
            K::Paren { inner } => self.expr_reads_admitted(inner, admitted),
            K::Unary { operand, .. } => self.expr_reads_admitted(operand, admitted),
            K::Binary { lhs, rhs, .. } => {
                self.expr_reads_admitted(lhs, admitted) || self.expr_reads_admitted(rhs, admitted)
            }
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.expr_reads_admitted(cond, admitted)
                    || self.expr_reads_admitted(then_e, admitted)
                    || self.expr_reads_admitted(else_e, admitted)
            }
            K::BitSelect { base, index } => {
                self.expr_reads_admitted(base, admitted)
                    || self.expr_reads_admitted(index, admitted)
            }
            K::PartSelect { base, msb, lsb } => {
                self.expr_reads_admitted(base, admitted)
                    || self.expr_reads_admitted(msb, admitted)
                    || self.expr_reads_admitted(lsb, admitted)
            }
            K::IndexedPart {
                base,
                offset,
                width,
                ..
            } => {
                self.expr_reads_admitted(base, admitted)
                    || self.expr_reads_admitted(offset, admitted)
                    || self.expr_reads_admitted(width, admitted)
            }
            K::Concat { parts } => any(parts),
            K::Replicate { count, value } => {
                self.expr_reads_admitted(count, admitted) || any(value)
            }
            K::Call { args, .. } | K::SysCall { args, .. } => any(args),
            K::Cast { target, expr } => {
                self.expr_reads_admitted(expr, admitted)
                    || matches!(target, ast::CastTarget::Size(sz) if self.expr_reads_admitted(sz, admitted))
            }
            _ => true,
        }
    }

    /// Does this declaration carry at least one ONCE-only initializer, resolved under
    /// the CURRENT prefix? The admission test for [`Self::emit_frame_static_prologue`].
    fn decl_has_static_once(&self, d: &ast::NetVarDecl) -> bool {
        d.names
            .iter()
            .filter_map(|decl| decl.init.as_ref())
            .any(|init| self.frame_static_init_once(d, init))
    }

    /// One declaration's ONCE-only initializers, lowered under the CURRENT prefix.
    fn emit_static_decl_inits(&mut self, b: &mut ProcessBuilder, d: &ast::NetVarDecl) {
        for decl in &d.names {
            let Some(init) = &decl.init else { continue };
            if !self.frame_static_init_once(d, init) {
                continue;
            }
            let stmt = ast::Stmt::Blocking {
                lhs: ast::Lvalue::Ident(ast::HierPath {
                    segments: vec![decl.name.clone()],
                    span: decl.name.span,
                }),
                delay: None,
                event: None,
                rhs: init.clone(),
                span: decl.span,
            };
            let saved = std::mem::replace(&mut self.lowering_decl_init, true);
            self.lower_stmt(b, &stmt);
            self.lowering_decl_init = saved;
            // The identity `emit_frame_local_inits` tests. Recorded HERE, at the one place
            // an initializer is actually lowered once-only, so the two emitters partition
            // the declarators instead of both answering a prefix-sensitive predicate.
            self.frame_hoisted_decls.insert(decl.name.span.lo);
        }
    }

    /// May this STATIC frame-local initializer be hoisted to the once-only prologue?
    ///
    /// Admitted by an ALLOWLIST of operand-only expression shapes over names that are
    /// either elaboration constants or other locals of this same frame. Everything
    /// else — a call (`int a = f2();`, loud E3009 at the per-activation emission and
    /// kept loud here), a system function, a pattern/`new` initializer, and above all
    /// a read of a FORMAL argument of this frame (or of a local whose own initializer was
    /// declined, transitively) and a read of a net OUTSIDE the frame — is declined and keeps the pre-slice
    /// per-activation emission. The outside read is declined because the two oracles
    /// disagree about it (measured: `int n; initial n = 9;` read by a static local
    /// initializer is `0` on iverilog and `9` on verilator), and a third answer is
    /// worse than either.
    fn frame_static_init_t0_safe(&self, e: &ast::Expr) -> bool {
        use ast::ExprKind as K;
        let all = |v: &[ast::Expr]| v.iter().all(|x| self.frame_static_init_t0_safe(x));
        match &e.kind {
            K::IntLit { .. } | K::RealLit { .. } | K::StrLit { .. } => true,
            K::Ident(p) => {
                p.segments.len() == 1 && self.frame_init_name_is_local_or_const(&p.segments[0].name)
            }
            K::Paren { inner } => self.frame_static_init_t0_safe(inner),
            K::Unary { operand, .. } => self.frame_static_init_t0_safe(operand),
            K::Binary { lhs, rhs, .. } => {
                self.frame_static_init_t0_safe(lhs) && self.frame_static_init_t0_safe(rhs)
            }
            K::Ternary {
                cond,
                then_e,
                else_e,
            } => {
                self.frame_static_init_t0_safe(cond)
                    && self.frame_static_init_t0_safe(then_e)
                    && self.frame_static_init_t0_safe(else_e)
            }
            K::BitSelect { base, index } => {
                self.frame_static_init_t0_safe(base) && self.frame_static_init_t0_safe(index)
            }
            K::PartSelect { base, msb, lsb } => {
                self.frame_static_init_t0_safe(base)
                    && self.frame_static_init_t0_safe(msb)
                    && self.frame_static_init_t0_safe(lsb)
            }
            K::IndexedPart {
                base,
                offset,
                width,
                ..
            } => {
                self.frame_static_init_t0_safe(base)
                    && self.frame_static_init_t0_safe(offset)
                    && self.frame_static_init_t0_safe(width)
            }
            K::Concat { parts } => all(parts),
            K::Replicate { count, value } => self.frame_static_init_t0_safe(count) && all(value),
            _ => false,
        }
    }

    /// Is this bare name safe to read from a t0 static-initializer write for the frame
    /// being lowered — an elaboration constant (parameter/localparam/genvar/enum label,
    /// which folds to the same value at any time), or a net of THIS frame (a sibling
    /// local, whose own initializer precedes this one in the same declaration-ordered
    /// flush)? A name that resolves to a net outside the frame is not.
    fn frame_init_name_is_local_or_const(&self, name: &str) -> bool {
        if self.lookup_scoped(name).is_some() {
            return true;
        }
        self.walk_scopes_key(name, |k| self.symbols.contains_key(k))
            .is_some_and(|k| {
                k.split('.').any(|seg| seg.starts_with("$func$"))
                    && !self.frame_nonhoistable.contains(&k)
            })
    }
}
