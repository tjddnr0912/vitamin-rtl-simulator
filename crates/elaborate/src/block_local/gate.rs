//! block-local ACCEPTANCE gates — the callee-effect resolver, the definite-assignment
//! entry point, the scope-leak check, and the diagnostics each of them emits.
//!
//! Split from `block_local.rs` (R17) to hold every module under the 1000-line cap.
//! These decide WHETHER a block-local can be flattened; `hoist.rs` does the
//! flattening.

use super::*;

/// R18 §3.3: does this call ACTUAL name the record variable whose SoA member net is
/// `name`?
///
/// An unpacked struct is fanned out by the PARSER into one net per member,
/// `$unp$<var>$<member>`, and `rm.count` is rewritten to it — but a call argument
/// stays the bare record name `rm`. So the definite-assignment walk, which tracks the
/// member net, saw `rsp_next(fd, rm)` as touching nothing: the write through the
/// formal was invisible (a false-loud) and, worse, so was the copy-IN read of an
/// `inout` (a latent unsoundness the same blindness hid).
///
/// The var part is matched EXACTLY (prefix `$unp$` + the actual's own identifier + a
/// `$`), so a name that merely looks similar cannot claim to be a member of it. Names
/// not of that shape answer `false`, leaving every non-SoA call decision byte-identical.
fn actual_is_record_of(arg: &ast::Expr, name: &str) -> bool {
    let ast::ExprKind::Ident(p) = &arg.kind else {
        return false;
    };
    if p.segments.len() != 1 {
        return false;
    }
    name.strip_prefix("$unp$")
        .and_then(|r| r.strip_prefix(p.segments[0].name.as_str()))
        .is_some_and(|r| r.starts_with('$'))
}

impl Elaborator<'_> {
    /// BL4 (round-19): resolve a callee NAME to its formal PORTS, in declaration
    /// order, for the block-local definite-assignment gate. A single-segment name is
    /// looked up in `func_table` then `task_table` (both hold the per-module AST def
    /// with `ports: Vec<TfPort>`). Returns `None` for a hierarchical `u.f` (the child
    /// module's ports are out of scope here), an unknown name, or a `$system` call —
    /// leaving the DA walk to treat the reference conservatively as a read.
    fn callee_ports(&self, callee: &ast::HierPath) -> Option<&[ast::TfPort]> {
        if callee.segments.len() != 1 {
            return None;
        }
        let nm = callee.segments[0].name.as_str();
        if let Some(f) = self.func_table.get(nm) {
            return Some(&f.ports);
        }
        if let Some(t) = self.task_table.get(nm) {
            return Some(&t.ports);
        }
        None
    }

    /// R19 §3.3: bind every ACTUAL of a call to the FORMAL DIRECTION it connects to.
    ///
    /// Positional actuals fill formals left to right; a `.formal(value)` names its own,
    /// and a positional actual may not follow a named one (IEEE 1800 §13.5.4). The two
    /// DA resolvers used to bail on the FIRST `NamedArg` they saw — so a call carrying
    /// one, which is exactly how a testbench leaves the other defaults alone
    /// (`run_scenario("D11", HASH_SHA2_256, '{}, msg, exp, .inject_rresp_error(1))`),
    /// answered `Unknown` and ended the walk, blaming a local several arguments to its
    /// left. Nothing about the named argument made the call unanalysable; the mapping
    /// was simply never written.
    ///
    /// `None` for anything not well-formed — an unresolvable callee, a name that is not
    /// a formal, a formal bound twice, a positional after a named one, more actuals
    /// than formals — leaving the caller exactly as conservative as it was.
    ///
    /// An OMITTED formal (and a valueless `.formal()`) binds its DEFAULT, an expression
    /// lowered in the caller's own scope, so it CAN read the flattened `name`. It is
    /// reported as an `Input` bind rather than dropped: a read is a read, whoever wrote
    /// the expression. (`default_binding_matches_decl_scope` is the separate, and
    /// stricter, question of whether that lowering is legitimate at all.)
    pub(crate) fn callee_arg_dirs<'a>(
        &'a self,
        callee: &ast::HierPath,
        args: &'a [ast::Expr],
    ) -> Option<Vec<(ast::PortDir, &'a ast::Expr)>> {
        Some(
            self.callee_arg_binds(callee, args)?
                .into_iter()
                .map(|(d, _, a)| (d, a))
                .collect(),
        )
    }

    /// R20 §3.2: [`Self::callee_arg_dirs`] keeping the whole FORMAL, not just its
    /// direction. The inout proof needs the formal's NAME to ask what the callee's body
    /// does to it, so the binding walk — the one place the IEEE §13.5.4 mapping lives —
    /// yields the port and `callee_arg_dirs` became a projection of it. One mapping, two
    /// views, so a future rule cannot be applied to one and missed by the other.
    ///
    /// The EFFECTIVE direction is carried alongside the port rather than read off it: a
    /// valueless `.formal()` binds the formal's DEFAULT, an expression in the caller's own
    /// scope, and that is an Input READ whatever the formal is declared to be.
    pub(crate) fn callee_arg_binds<'a>(
        &'a self,
        callee: &ast::HierPath,
        args: &'a [ast::Expr],
    ) -> Option<Vec<(ast::PortDir, &'a ast::TfPort, &'a ast::Expr)>> {
        let ports = self.callee_ports(callee)?;
        let mut out: Vec<(ast::PortDir, &'a ast::TfPort, &'a ast::Expr)> =
            Vec::with_capacity(ports.len());
        let mut bound = vec![false; ports.len()];
        let mut pos = 0usize;
        let mut seen_named = false;
        for a in args {
            if let ast::ExprKind::NamedArg { formal, value } = &a.kind {
                seen_named = true;
                let idx = ports.iter().position(|p| p.name.name == formal.name)?;
                if std::mem::replace(&mut bound[idx], true) {
                    return None;
                }
                match value {
                    Some(v) => out.push((ports[idx].dir, &ports[idx], v)),
                    None => out.push((
                        ast::PortDir::Input,
                        &ports[idx],
                        ports[idx].default.as_ref()?,
                    )),
                }
            } else {
                if seen_named || pos >= ports.len() || std::mem::replace(&mut bound[pos], true) {
                    return None;
                }
                out.push((ports[pos].dir, &ports[pos], a));
                pos += 1;
            }
        }
        for (p, b) in ports.iter().zip(bound) {
            if !b {
                out.push((ast::PortDir::Input, p, p.default.as_ref()?));
            }
        }
        Some(out)
    }

    /// BL4 (round-19): true iff the call `callee(args)` DEFINITELY (whole-var) WRITES
    /// `name` through a PURE OUTPUT actual and READS `name` at NO position of the same
    /// call. This is the resolver threaded into `automatic_local_definitely_assigned` /
    /// `da_stmt` (see [`crate::da::OutActualWrites`]). Soundness (this is an ACCEPT gate
    /// — a wrong "assigned" reads a leftover/X value instead of erroring = silent-wrong):
    ///   * ONLY a bare whole-var `Ident(name)` at an OUTPUT formal position counts as a
    ///     write (copy-out only, no copy-in). A SELECT actual `name[i]` is a PARTIAL
    ///     write (the other bits stay unwritten) → NOT a definite whole assignment; it
    ///     is caught as a READ below and blocks the accept.
    ///   * An INOUT actual is NOT a write here: its copy-IN reads `name`'s current value
    ///     (the leftover on the v1 flatten ≠ a fresh automatic's default), so an inout
    ///     `name` is a READ (blocks the accept unless `name` is already assigned).
    ///   * `name` at ANY input actual, or inside a select index of any actual, or at an
    ///     arg beyond the resolved formals, is a READ.
    ///   * Named args (can't map positionally) or an unresolvable callee → `false`.
    ///
    /// The verdict is `writes && !reads`: a genuine output-write with no shadow read.
    ///
    /// R20 §3.2 lifts the INOUT clause above, but only on a PROOF (see
    /// [`Self::inout_copy_in_is_dead`]) — the copy-in still reads, it is just no longer
    /// OBSERVABLE when the callee overwrites the whole formal before looking at it.
    fn call_out_actual_writes(
        &self,
        callee: &ast::HierPath,
        args: &[ast::Expr],
        name: &str,
    ) -> bool {
        let Some(binds) = self.callee_arg_binds(callee, args) else {
            return false;
        };
        let mut writes = false;
        let mut reads = false;
        for (dir, port, arg) in binds {
            // A clean whole-var OUTPUT actual `name` at an OUTPUT formal — copy-out only.
            // R18 §3.3: `name` may be a MEMBER NET of an unpacked struct
            // (`$unp$rm$count`) whose actual still names the record (`rm`) — the SoA
            // fan-out is a parser rewrite that never reached the call arguments. Both
            // spellings denote the same storage here.
            let whole_var = matches!(&arg.kind, ast::ExprKind::Ident(p)
                    if p.segments.len() == 1 && p.segments[0].name == name)
                || actual_is_record_of(arg, name);
            let clean_output_whole = matches!(dir, ast::PortDir::Output) && whole_var;
            // R20 §3.2: an INOUT whole-var actual whose copy-in the callee cannot observe.
            let dead_inout_whole = matches!(dir, ast::PortDir::Inout)
                && whole_var
                && self
                    .callee_side_member_name(arg, &port.name.name, name)
                    .is_some_and(|f| self.inout_copy_in_is_dead(callee, &f));
            if clean_output_whole || dead_inout_whole {
                writes = true;
            } else if whole_var || !expr_no_ref(arg, name) {
                // Any OTHER reference to `name` (input actual, inout copy-in, select
                // index, partial-write select base, arg beyond arity, OR a same-call
                // member / method read `name.field` / `name.size()`) is a read that
                // observes the copy-IN value before the output copy-OUT. Use the
                // CONSERVATIVE `expr_no_ref` (any unvetted form ⇒ "may reference"), NOT
                // `expr_reads_ident` — the latter is the known under-detecting walker
                // (single-seg only, no `MethodCall`/`Dist`/… arm), so a co-arg
                // `name.field` (a multi-seg ident) / `name.method()` would slip through
                // and let the accept gate skip the loud → the flattened net's leftover
                // is read on re-entry = silent-wrong (BL4 adversarial-review finding;
                // latent today since member/method-bearing OUTPUT-formal types are
                // otherwise loud, but this keeps the gate sound as support widens).
                reads = true;
            }
        }
        writes && !reads
    }

    /// BL4 (round-19): [`automatic_local_definitely_assigned`] with this Elaborator's
    /// output-actual resolver ([`Self::call_out_actual_writes`]) bound in — so a call
    /// whose OUTPUT actual is `name` (unconditionally evaluated) is seen as a definite
    /// assignment, not a read-before-write. `&self` (immutable) — the returned `bool`
    /// releases the borrow before the surrounding `&mut self` hoist continues.
    /// `elem_bounds` is the declared index range when `name` is a single-dimension
    /// FIXED unpacked array (R16 §3.3) — the walk then accepts a complete set of
    /// straight-line constant-index element writes as the first whole write. Computed
    /// by the caller because folding a dimension needs `&mut self`.
    /// `sole_writer` says whether `stmts` is the ONLY code that can write the
    /// flattened net (see [`automatic_local_definitely_assigned`]) — `true` for a
    /// freshly declared block-local, `false` when the net is shared with a same-named
    /// block-local in another block.
    pub(crate) fn block_local_definitely_assigned(
        &self,
        stmts: &[ast::Stmt],
        name: &str,
        elem_bounds: Option<(i64, i64)>,
        sole_writer: bool,
        bit_width: Option<u32>,
    ) -> Result<(), crate::da::DaGiveUp> {
        automatic_local_definitely_assigned(
            stmts,
            name,
            &crate::da::DaCtx {
                out_writes: &|cn, args, nm| self.call_effect(cn, args, nm),
                suspends: &|cn| self.call_may_suspend(cn, Self::CALL_INERT_DEPTH),
                loop_once: &|st| self.loop_runs_once(st),
                sole: sole_writer,
            },
            elem_bounds,
            bit_width,
        )
    }

    /// R17 §3.3 / §4.2: the follow-on `note:` naming WHERE the definite-assignment
    /// walk stopped and WHY.
    ///
    /// The error above it is anchored at the DECLARATION, which is the right place to
    /// say "this declaration cannot be supported" and the wrong place to look for the
    /// cause. The round-17 report is the evidence: 21 diagnostics with exact
    /// locations, and not one of them reducible, because the location named a
    /// declaration that was fine. The construct that ended the scan can be several
    /// statements later, or — when the scan follows a call into a callee body —
    /// in another function in another file.
    pub(crate) fn note_da_gave_up(&mut self, nm: &str, g: crate::da::DaGiveUp) {
        self.note_at(
            MsgCode::ElabUnsupported,
            g.span,
            &format!(
                "definite-assignment for `{nm}` stopped here: {}. Everything after this \
                 point is treated as if `{nm}` were still unwritten",
                g.what
            ),
        );
    }

    /// R17 §3.3: the `automatic` per-entry-lifetime rejection and, when the cause was
    /// the definite-assignment walk rather than an initializer, the note that says
    /// where it stopped.
    pub(crate) fn deny_per_entry_lifetime(
        &mut self,
        nm: &str,
        gave_up: Option<crate::da::DaGiveUp>,
    ) {
        self.error(
            MsgCode::ElabUnsupported,
            &format!(
                "an `automatic` block-local `{nm}` whose per-entry lifetime differs from \
                 static (an initializer, or a read before its first write) is unsupported \
                 in a procedural block (v1 flattens block-locals to one static net); \
                 assign it before use, or drop `automatic`",
            ),
        );
        if let Some(g) = gave_up {
            self.note_da_gave_up(nm, g);
        }
    }

    /// R16 §3.3: the declared index bounds of a single-dimension FIXED unpacked array
    /// declarator, or `None` for a scalar, a dynamic/queue/assoc dim, a multi-dim
    /// array, or a bound that does not fold.
    pub(crate) fn fixed_elem_bounds(&mut self, n: &ast::DeclName) -> Option<(i64, i64)> {
        if n.unpacked.len() != 1 {
            return None;
        }
        self.fixed_dim_bounds(&n.unpacked[0])
    }

    /// R16 §3.2: how far the callee-body walk chases nested user calls before giving
    /// up. A cycle (`a` calls `b` calls `a`) is bounded by this counter rather than a
    /// visited-set, which keeps the walk `&self`-pure; the budget only ever costs
    /// precision (an exhausted budget answers "may touch" and the local stays loud).
    pub(crate) const CALL_INERT_DEPTH: u32 = 8;

    /// R16 §3.2: the three-way verdict [`crate::da::CallEffect`] for one call, and the
    /// resolver threaded into the definite-assignment walk.
    ///
    /// `Writes` is the pre-existing pure-output-actual case, unchanged. `Inert` is the
    /// new one: a call proven to touch `name` at NO position, which the walk can step
    /// over. Proving it needs the callee's BODY, not just its signature — v1 flattens
    /// block-locals into the module namespace, so a body can name the flattened net
    /// with neither the call head nor any argument mentioning it (verified: a
    /// `task poke; t.a = 99; endtask` writes a block-local `a` through a hierarchical
    /// self-path). That hazard is exactly why a bare `show(x);` statement was left
    /// unvetted; it is answered here rather than assumed away.
    pub(crate) fn call_effect(
        &self,
        callee: &ast::HierPath,
        args: &[ast::Expr],
        name: &str,
    ) -> crate::da::CallEffect {
        use crate::da::CallEffect as E;
        if self.call_out_actual_writes(callee, args, name) {
            return E::Writes;
        }
        if self.call_is_inert(callee, args, name, Self::CALL_INERT_DEPTH) {
            return E::Inert;
        }
        if self.call_only_reads(callee, args, name) {
            return E::Reads;
        }
        E::Unknown
    }

    /// R17 §3.2: can `callee(args)` be proven to READ `name` without ever WRITING it?
    ///
    /// Same body obligation as [`Self::call_is_inert`] — the callee must be unable to
    /// reach the flattened net on its own — but the ACTUALS are allowed to mention
    /// `name` as long as every mention sits at an `input` formal, whose copy-in is a
    /// pure read (IEEE 1800 §13.5.1). That is the difference between "this call is
    /// invisible to `name`" and "this call looks at `name`", and it is the fact the
    /// never-written argument needs: a local deliberately left unassigned and passed
    /// by value is not a read-before-write, because there is no write for it to be
    /// before.
    ///
    /// Restricted to a SINGLE-segment callee resolved in this module's
    /// `func_table`/`task_table`. A two-segment path is treated elsewhere as a
    /// container-method enable, but it can also be a hierarchical task call whose
    /// formals are not visible here — and an `output` formal reached that way would
    /// turn this proof into a silent-wrong, so it is not claimed.
    fn call_only_reads(&self, callee: &ast::HierPath, args: &[ast::Expr], name: &str) -> bool {
        // A two-segment path whose HEAD IS `name` is unambiguously a method on our own
        // local: `name` is a block-local VARIABLE, so it can never also be the instance
        // name a hierarchical `inst.task(…)` would need. That removes the one hazard
        // that keeps the rest of this function single-segment, and leaves only "does
        // this method mutate its receiver".
        if callee.segments.len() == 2 && callee.segments[0].name == name {
            return container_method_is_pure(&callee.segments[1].name)
                && args.iter().all(|a| expr_no_ref_deep(a, name));
        }
        if callee.segments.len() != 1 {
            return false;
        }
        let Some(binds) = self.callee_arg_dirs(callee, args) else {
            return false;
        };
        // Every actual that MAY mention `name` must be at an `input` formal (a copy-in
        // is a pure read). `callee_arg_dirs` has already rejected any binding it could
        // not map, so an unmapped position cannot reach here.
        for (dir, arg) in binds {
            if expr_no_ref_deep(arg, name) {
                continue;
            }
            if !matches!(dir, ast::PortDir::Input) {
                return false;
            }
        }
        self.callee_body_cannot_touch(callee, name, Self::CALL_INERT_DEPTH)
    }

    /// R16 §3.2: can `callee(args)` be proven to touch `name` at no position?
    ///
    /// Three obligations, all conservative (any doubt answers `false`):
    ///   1. every ACTUAL is ref-free under the all-segments rule;
    ///   2. the callee RESOLVES — a single-segment name in this module's
    ///      `func_table`/`task_table`, or a 2-segment container-method enable whose
    ///      receiver is not `name` (`q.delete();` touches only `q`);
    ///   3. the resolved BODY (its declarations' initializers included) is clean under
    ///      [`stmt_no_ref_deep`], which recurses here for any call the body itself
    ///      makes.
    ///
    /// A hierarchical callee, a `$system` task, an unknown name, or an exhausted depth
    /// budget all answer `false` and leave the local loud — the same outcome as before
    /// this existed.
    /// R18-X1: can enabling `callee` let simulation TIME ADVANCE (transitively)?
    ///
    /// The time-side twin of [`Self::call_is_inert`], and deliberately independent of
    /// it: a callee can be provably inert with respect to a name and still suspend,
    /// and suspending is what hands the one shared flattened net to another block.
    /// Wrapping `@(posedge clk)` in a one-line helper used to hide the suspend from
    /// the shared-net rule entirely — a silent-wrong measured identical at `c8ad2b4`
    /// and `46b9816` (vita `A v=99` / iverilog `A v=1`, exit 0).
    ///
    /// POLARITY: every unknown answers `true` (may suspend), because the only reader
    /// is a REJECT gate. Hierarchical callee, unresolvable name, exhausted budget —
    /// all `true`. Functions are walked too even though IEEE 1800 §13.4.4 forbids
    /// them from consuming time: costs nothing and needs no trust in the parser
    /// having enforced it.
    pub(crate) fn call_may_suspend(&self, callee: &ast::HierPath, depth: u32) -> bool {
        if callee.segments.len() != 1 || depth == 0 {
            return true;
        }
        let nm = callee.segments[0].name.as_str();
        let body = if let Some(t) = self.task_table.get(nm) {
            &t.body
        } else if let Some(f) = self.func_table.get(nm) {
            &f.body
        } else {
            return true;
        };
        crate::da::body_advances_time(body, &|cn| self.call_may_suspend(cn, depth - 1))
    }

    fn call_is_inert(
        &self,
        callee: &ast::HierPath,
        args: &[ast::Expr],
        name: &str,
        depth: u32,
    ) -> bool {
        if !args.iter().all(|a| expr_no_ref_deep(a, name)) {
            return false;
        }
        // A container-method enable (`q.delete();`, `s.putc(i, c);`) touches only its
        // receiver and arguments — no user body to walk.
        if callee.segments.len() == 2 {
            return callee.segments.iter().all(|s| s.name != name);
        }
        // R19: an OMITTED formal's DEFAULT is lowered in the caller's scope too, so a
        // call whose written-out arguments never mention `name` can still read it. The
        // args loop above cannot see that — the expression is on the callee's port.
        if !self
            .callee_arg_dirs(callee, args)
            .is_some_and(|b| b.iter().all(|(_, a)| expr_no_ref_deep(a, name)))
        {
            return false;
        }
        self.callee_body_cannot_touch(callee, name, depth)
    }

    /// R16 §3.2 obligation 3, split out of [`Self::call_is_inert`] so R17 §3.2's
    /// read-only verdict can reuse it: the callee's BODY (its declarations'
    /// initializers included) provably cannot reach the flattened `name`.
    ///
    /// Nothing about the ACTUALS is claimed here — that is each caller's own
    /// obligation, and the two callers state different ones (ref-free actuals for
    /// `Inert`, `input`-only actuals for `Reads`).
    pub(crate) fn callee_body_cannot_touch(
        &self,
        callee: &ast::HierPath,
        name: &str,
        depth: u32,
    ) -> bool {
        if callee.segments.len() != 1 || depth == 0 {
            return false;
        }
        let nm = callee.segments[0].name.as_str();
        if nm == name {
            return false;
        }
        let (body, body_decls) = if let Some(f) = self.func_table.get(nm) {
            (&f.body, &f.body_decls)
        } else if let Some(t) = self.task_table.get(nm) {
            (&t.body, &t.body_decls)
        } else {
            return false;
        };
        // A formal or body-local of the callee named `name` SHADOWS the flattened net,
        // so references inside are not to our local — but proving which is which needs
        // the callee's scope, so treat the whole body as unvettable instead. Loud, not
        // wrong.
        if body_decls
            .iter()
            .flat_map(|d| d.names.iter())
            .any(|n| n.name.name == name)
        {
            return false;
        }
        body_decls
            .iter()
            .flat_map(|d| d.names.iter())
            .all(|n| n.init.as_ref().is_none_or(|e| expr_no_ref_deep(e, name)))
            && stmt_no_ref_deep(body, name, &|cn, ar, nm2| {
                self.call_is_inert(cn, ar, nm2, depth - 1)
            })
    }

    /// §6.21: a STATIC block-local's initializer runs ONCE at time zero, before any
    /// process starts; an `automatic` sibling's runs on each BLOCK ENTRY. So a static
    /// initializer that reads a per-entry sibling reads it at a time the automatic has
    /// not been initialized — and worse, anything the static initializer WRITES back
    /// into that sibling (an `inout`/`output` copy-out: `int z = f(c);`) is then clobbered
    /// by the entry re-init. Both values are wrong and neither is recoverable, so this
    /// combination is loud.
    ///
    /// Found while lifting the fork restriction (§4.5.248): the whole fork arm used to be
    /// loud for a different reason, which masked this. The identical NON-fork shape was
    /// already silently printing `c=5` where the copy-out said 6 — so this is a
    /// silent-wrong being raised to loud, not a capability being taken away.
    pub(crate) fn deny_static_init_reading_per_entry(
        &mut self,
        decls: &[ast::NetVarDecl],
        span: ast::Span,
    ) {
        let mut per_entry = self.per_entry_in_scope.clone();
        if let Some(here) = self.per_entry_block_locals.get(&span.lo) {
            per_entry.extend(here.iter().cloned());
        }
        if per_entry.is_empty() {
            return;
        }
        let per_entry = &per_entry;
        // POLARITY (review F4): this is a REJECT gate, so it uses the POSITIVE walker.
        // `expr_no_ref` answers "may reference" for anything it has not vetted, which is
        // right for an accept gate and turns every unvetted initializer into a rejection
        // here — `int idx = pkg::BASE;` beside any `automatic` sibling was rejected, with
        // a message naming a variable it never mentions.
        // Carry each hit's own DECL span so the diagnostic points at the declaration
        // rather than at nothing (review F9 — this gate runs at block level, before the
        // per-decl `cur_span` anchor in the hoist loop).
        let hits: Vec<(String, String, ast::Span)> = decls
            .iter()
            .filter(|d| d.lifetime != Some(true))
            .flat_map(|d| d.names.iter().map(move |n| (n, d.span)))
            .filter(|(n, _)| !per_entry.contains(&n.name.name))
            .filter_map(|(n, sp)| {
                let init = n.init.as_ref()?;
                let hit = per_entry.iter().find(|pn| expr_definitely_refs(init, pn))?;
                Some((n.name.name.clone(), hit.clone(), sp))
            })
            .collect();
        for (stat, auto, sp) in hits {
            let saved = self.cur_span.replace(sp);
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "the STATIC block-local `{stat}`'s initializer reads the `automatic` \
                     block-local `{auto}` in scope here; a static initializer \
                     runs once at time 0 while an `automatic` is initialized on each block \
                     entry, so `{auto}` has no value yet there (and a write back into it \
                     through an output/inout formal is overwritten by the entry \
                     initialization) — declare `{stat}` `automatic`, or move the \
                     initialization into the block body"
                ),
            );
            self.cur_span = saved;
        }
    }

    /// Allocate this frame function's nets (formals, return-var, body_decls — in
    /// that slot order) under a synthetic `$func$<name>` scope, push a placeholder
    /// `FuncDef` + the complete `FuncMeta`, and record the name→FuncId divert.
    /// Reserve a frame body's block-local declarations (`begin int tmp; …`) under
    /// the current `$func$<name>` scope, in source order, AFTER the formals /
    /// return / top-level `body_decls`. A name already reserved (a formal, the
    /// return var, a top-level local, or an earlier same-named block) is COALESCED
    /// (shared net) — like the process-body hoist for two sequential blocks reusing
    /// IEEE §6.21: a local declared in a nested `begin…end` is visible only within
    /// that block. vita keeps a function/task/procedural body's block-locals in a
    /// FLAT per-body table, so a reference OUTSIDE the declaring block would
    /// silently resolve to the block-local instead of the lexically-correct outer
    /// binding (a module variable, a formal, or an enclosing local). Proper
    /// per-block scope lowering is a large follow-on (ROADMAP §4.5.18); until then,
    /// detect that exact leak and reject it LOUD (correct-or-loud) rather than
    /// produce a silent-wrong. A block-local referenced only inside its own block
    /// resolves correctly and is left untouched (no diagnostic, byte-identical).
    pub(crate) fn check_block_local_scope_leaks(&mut self, body: &ast::Stmt) {
        let mut nested = Vec::new();
        gather_nested_block_locals(body, true, &mut nested);
        for (bspan, names) in &nested {
            for (n, dspan) in names {
                // A `$blk$`-SCOPED name is not in the flat table this gate is about.
                // The paragraph above states the hazard exactly — "vita keeps a body's
                // block-locals in a FLAT per-body table, so a reference OUTSIDE the
                // declaring block would silently resolve to the block-local" — and that
                // is false once the declaration owns a `$blk$<lo>` net: the outside
                // reference walks past a scope that does not hold the name and lands on
                // the outer binding, which is the answer both oracles give.
                //
                // ⚠️ Keyed on `scoped_block_locals` rather than on "does it shadow a
                // module name", because that map is what actually decides whether the
                // scope EXISTS. It is populated for MODULE PROCESS bodies only
                // (`compute_scoped_block_locals` walks `for_each_proc`), so a
                // function/task body — where the flat table is still the whole story —
                // matches nothing here and keeps the diagnostic unchanged.
                if self
                    .scoped_block_locals
                    .get(&bspan.lo)
                    .is_some_and(|set| set.contains(n))
                {
                    continue;
                }
                let Some(hit) = stmt_refs_ident_outside(body, *bspan, n) else {
                    continue;
                };
                // R17 §4.1: anchor the error at the DECLARATION (what the message asks
                // the user to rename) and the note at the OUTSIDE REFERENCE (what makes
                // it illegal). Neither alone is enough — the two are routinely in
                // different statements, and this diagnostic used to print no location
                // at all.
                let saved = self.cur_span.replace(*dspan);
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "block-local `{n}` is referenced outside its `begin…end` block; \
                         vita cannot resolve this to the outer `{n}` yet (it would silently \
                         read the block-local) — rename the block-local or hoist its \
                         declaration to the enclosing scope"
                    ),
                );
                self.note_at(
                    MsgCode::ElabUnsupported,
                    hit,
                    &format!("`{n}` is referenced here, outside the block that declares it"),
                );
                self.cur_span = saved;
            }
        }
    }
}
