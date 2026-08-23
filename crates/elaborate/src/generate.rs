//! generate blocks — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

// ════════════════════════════════════════════════════════════════════
//  v4 — GENERATE unrolling (GenerateConstruct → flat SimIr at elab time)
// ════════════════════════════════════════════════════════════════════
//
// A generate construct is expanded at ELABORATION time: a generate-for with N
// iterations becomes N copies of its body in the flat SimIr (genvar bound to
// each iteration value); a generate-if/case selects exactly one branch. Nothing
// generate-related survives into sim-ir — the genvar is an elaboration-only
// integer (it lives in `self.params`, never `self.nets`).
//
// PHASE SPLIT (the determinism contract): the existing flat-module lowering
// relies on net-decl order (pass 4) < cont-assign/proc order (pass 7) < child
// instance recursion (pass 8). A generate block mixes all three. So we re-walk
// the gen-item tree once per phase, doing only the matching kind of work. The
// unroll arithmetic (const-eval of init/cond/step) is pure and side-effect-free,
// so every phase reproduces the SAME genvar sequence and the SAME `label[idx]`
// prefixes — nets land entirely in the Nets walk (before any Logic), Logic
// before Instances, exactly mirroring the flat-module pass order.

/// Which slice of work a generate walk performs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum GenPhase {
    /// Create NetVar nets only (so they sit in the parent's contiguous slice).
    Nets,
    /// Collect + flush §6.8 variable decl-init pre-sweeps (unpacked-array `'{…}`
    /// patterns and non-constant scalars) as t0 `initial` blocks, ONE per generate
    /// scope. Runs AFTER the Nets walk (nets exist) and BEFORE the Logic walk. A constant
    /// scalar is collected here like any other initializer (§4.5.257). Queue /
    /// dyn-array / string decl-inits in a generate scope stay a LOUD reject (their
    /// handle net is not created here — `allow_string_init` is `false`), a
    /// documented follow-on.
    VarInit,
    /// Lower cont-assigns + processes only (nets already created in the Nets walk).
    Logic,
    /// Recurse into child module instances only (after the parent net slice is final).
    Instances,
}

impl Elaborator<'_> {
    /// True iff `seg` is a GENERATE-block scope segment (`label[idx]`), as opposed
    /// to an instance-boundary segment (a plain identifier). Generate prefixes
    /// always carry the `[idx]` suffix, so a `[` unambiguously marks them.
    pub(crate) fn is_gen_scope_segment(seg: &str) -> bool {
        seg.contains('[')
    }

    /// Run `f` with `cur_prefix` temporarily extended by `seg` (a gen-block
    /// `label[idx]` segment). Restores the prefix on return. Genvar bindings in
    /// `self.params` are NOT touched here (the caller manages those).
    pub(crate) fn with_scope<R>(&mut self, seg: &str, f: impl FnOnce(&mut Self) -> R) -> R {
        let new_prefix = if self.cur_prefix.is_empty() {
            seg.to_string()
        } else {
            format!("{}.{}", self.cur_prefix, seg)
        };
        let saved = std::mem::replace(&mut self.cur_prefix, new_prefix);
        let r = f(self);
        self.cur_prefix = saved;
        r
    }

    /// Unroll/select a list of GenItems at elaboration time, in deterministic
    /// order. `phase` selects which lowering work to do (see [`GenPhase`]).
    /// `depth` is the nesting guard. Genvars bind into `self.params` (like a
    /// param) so `const_eval_in_scope` resolves them; `with_scope` gives each
    /// loop iteration its `label[idx].` namespace.
    /// `is_scope` distinguishes the two things this function serves.
    ///
    /// §4.5.263: a `generate … endgenerate` REGION is purely syntactic (IEEE 1800 §27.3)
    /// — items written directly in it, outside any `if`/`for`/`case`, are ordinary module
    /// items. A generate BLOCK BODY is a scope. This one function is called for both, so
    /// giving it a rank scope, the `in_generate_body` flag and its own flush made a region
    /// behave like a block: `generate int mv = g.gv; endgenerate` read 0 instead of the
    /// block's 7, and a bare instance in a region initialized before the block beside it.
    /// Regions pass `false` and are transparent, exactly as they were before this series.
    pub(crate) fn elaborate_generate_scoped(
        &mut self,
        items: &[ast::GenItem],
        phase: GenPhase,
        depth: u32,
        map: &ModuleMap<'_>,
        is_scope: bool,
    ) {
        if depth > GENERATE_DEPTH_CAP {
            // depth guard reported ONCE (in the Nets phase) to avoid 3× dup.
            if phase == GenPhase::Nets {
                self.error(
                    MsgCode::ElabUnsupported,
                    "generate nesting too deep (deferred)",
                );
            }
            return;
        }
        // VarInit: isolate THIS scope-level's pending list so a nested block's
        // flush drains only its own inits (not ours), and flush at the end while
        // the current scope prefix is still active — so each collected bare-name
        // lvalue (`arr`) resolves to the scoped net (`blk[idx].arr`), and the
        // synthesized `initial` lands in this scope. `pending_var_inits` is empty
        // outside the module/interface flush windows, but the save/restore keeps
        // sibling scopes independent regardless. (No-op for other phases.)
        // §4.5.255: a generate body is a SCOPE OF ITS OWN to iverilog even where vita
        // mints no prefix segment for it (a `case` arm, an unlabeled `if`/`begin`), and it
        // initializes before the enclosing module. The flag travels with each initializer
        // this walk collects so the enclosing flush can tell "declared in a child generate
        // body" from "declared in my own body" — the two go on opposite sides of the
        // module-scope initializers, and they are indistinguishable by prefix alone.
        // The rank slot a generate body occupies depends on its PARENT's kind, so it is
        // read BEFORE the flag flips.
        if !is_scope {
            // A REGION: transparent. Its items belong to the enclosing scope, so no rank
            // scope, no `in_generate_body`, no isolated pending list and no flush of its
            // own — the enclosing scope's flush takes them, in declaration order with its
            // own items.
            for item in items {
                self.elaborate_gen_item(item, phase, depth, map);
            }
            return;
        }
        let slot = self.rank_slot_for_generate();
        let saved_in_gen = std::mem::replace(&mut self.in_generate_body, true);
        let saved_pending =
            (phase == GenPhase::VarInit).then(|| std::mem::take(&mut self.pending_var_inits));
        self.with_rank_scope(slot, |s| {
            for item in items {
                s.elaborate_gen_item(item, phase, depth, map);
            }
            if saved_pending.is_some() {
                s.flush_block_local_inits();
            }
        });
        if let Some(outer) = saved_pending {
            self.pending_var_inits = outer;
        }
        self.in_generate_body = saved_in_gen;
    }

    pub(crate) fn elaborate_gen_item(
        &mut self,
        item: &ast::GenItem,
        phase: GenPhase,
        depth: u32,
        map: &ModuleMap<'_>,
    ) {
        match item {
            // ── generate-for: bind genvar, unroll ascending ──────────
            ast::GenItem::For {
                init,
                cond,
                step,
                label,
                body,
                ..
            } => {
                let gv_key = self.fq(&init.lvalue.name);

                // INIT value, const-eval'd in the current scope.
                let Some(start) = self.const_eval_in_scope(&init.value) else {
                    if phase == GenPhase::Nets {
                        self.error(
                            MsgCode::ElabUnresolvedName,
                            "generate-for init is not a constant",
                        );
                    }
                    return;
                };

                // Save any prior binding of this name (an outer param/genvar of the
                // same identifier) and seed the genvar.
                let saved = self.params.insert(gv_key.clone(), start);
                // A genvar IS a signed 32-bit integer (IEEE 1800 §27.4), and the
                // const domain's sign model reads `param_meta` — with no entry it
                // answered UNSIGNED, so `2 ** (g - 1)` at g = 0 masked −1 into
                // 4294967295 and the fold overflowed to a LOUD reject where every
                // oracle says 0. Seeding the shape here (saved/restored in lockstep
                // with the value) keeps ONE spelling of "what is a genvar" instead
                // of a special case in each predicate. Narrow by construction: for a
                // NON-negative 32-bit value the signed and unsigned readings are the
                // same bits, so only an expression whose intermediate goes negative
                // can move at all.
                let saved_meta = self.param_meta.insert(gv_key.clone(), (32, true));

                let mut idx_count: u32 = 0;
                loop {
                    // cond folded WITH the genvar bound (so `i < N` resolves).
                    let keep = match self.const_truth_in_scope(cond) {
                        Some(c) => c,
                        None => {
                            if phase == GenPhase::Nets {
                                self.error(
                                    MsgCode::ElabUnresolvedName,
                                    "generate-for condition is not a constant",
                                );
                            }
                            break;
                        }
                    };
                    if !keep {
                        break;
                    }
                    if idx_count >= GENERATE_UNROLL_CAP {
                        if phase == GenPhase::Nets {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "generate-for exceeds the unroll cap (possible infinite loop)",
                            );
                        }
                        break;
                    }

                    // The genvar VALUE (not a 0-based counter) indexes the block
                    // name, so `for(i=2;i<5;…)` yields `[2],[3],[4]` per Verilog.
                    let iter_val = *self.params.get(&gv_key).unwrap_or(&0);
                    let lbl = label.as_ref().map(|l| l.name.as_str()).unwrap_or("genblk");
                    let block_prefix = format!("{lbl}[{iter_val}]");

                    self.with_scope(&block_prefix, |me| {
                        me.elaborate_generate_scoped(body, phase, depth + 1, map, true);
                    });

                    // step: fold (with genvar bound) → rebind the genvar.
                    let Some(next) = self.const_eval_in_scope(&step.value) else {
                        if phase == GenPhase::Nets {
                            self.error(
                                MsgCode::ElabUnresolvedName,
                                "generate-for step is not a constant",
                            );
                        }
                        break;
                    };
                    // STALL GUARD (verdict M1): the genvar VALUE namespaces each
                    // iteration's block (`label[iter_val]`). If the step does NOT
                    // advance it (`next == iter_val`, e.g. `i = i`), every iteration
                    // reuses the SAME prefix and collides at `add_net`, emitting one
                    // duplicate-decl error PER iteration up to the unroll cap (~4k
                    // spurious diagnostics). Detect the non-progressing step and stop
                    // with ONE diagnostic. (A value that merely repeats LATER — a
                    // non-monotonic cycle — is still bounded by the unroll cap;
                    // correctness intact, diagnostics less clean. Residual risk R3.)
                    if next == iter_val {
                        if phase == GenPhase::Nets {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "generate-for genvar does not advance (step leaves it unchanged)",
                            );
                        }
                        break;
                    }
                    self.params.insert(gv_key.clone(), next);
                    idx_count += 1;
                }

                // restore the prior binding (siblings/ancestors unaffected).
                match saved {
                    Some(v) => {
                        self.params.insert(gv_key.clone(), v);
                    }
                    None => {
                        self.params.remove(&gv_key);
                    }
                }
                match saved_meta {
                    Some(m) => {
                        self.param_meta.insert(gv_key, m);
                    }
                    None => {
                        self.param_meta.remove(&gv_key);
                    }
                }
            }

            // ── generate-if: const-eval cond, take ONE branch ────────
            ast::GenItem::If {
                cond,
                then_b,
                else_b,
                label,
                ..
            } => {
                let taken = match self.const_truth_in_scope(cond) {
                    Some(c) => c,
                    None => {
                        if phase == GenPhase::Nets {
                            self.error(
                                MsgCode::ElabUnresolvedName,
                                "generate-if condition is not a constant",
                            );
                        }
                        return;
                    }
                };
                let body = if taken { then_b } else { else_b };
                // An `if` BODY is a scope even unlabeled — measured: its content
                // initializes before the enclosing module's own variables.
                self.elaborate_gen_scoped(label, body, phase, depth, map, true);
            }

            // ── generate-case: const-eval scrutinee, match ONE item ──
            ast::GenItem::Case {
                scrutinee, items, ..
            } => {
                let Some(scrut) = self.const_eval_in_scope(scrutinee) else {
                    if phase == GenPhase::Nets {
                        self.error(
                            MsgCode::ElabUnresolvedName,
                            "generate-case scrutinee is not a constant",
                        );
                    }
                    return;
                };
                // first Match whose label const-equals scrut wins; else Default.
                let mut chosen: Option<&[ast::GenItem]> = None;
                let mut default: Option<&[ast::GenItem]> = None;
                'scan: for ci in items {
                    match ci {
                        ast::GenCaseItem::Match { labels, body, .. } => {
                            for lab in labels {
                                if self.const_eval_in_scope(lab) == Some(scrut) {
                                    chosen = Some(body);
                                    break 'scan;
                                }
                            }
                        }
                        ast::GenCaseItem::Default { body, .. } => {
                            default = Some(body);
                        }
                    }
                }
                if let Some(body) = chosen.or(default) {
                    self.elaborate_generate_scoped(body, phase, depth + 1, map, true);
                }
            }

            // ── named/unnamed begin…end block inside generate ────────
            ast::GenItem::Block { label, items, .. } => {
                // §4.5.264: a bare `begin…end` in a gen-item list is the ANACHRONISTIC
                // SURROUND (iverilog warns and treats it as syntax) — `parse_gen_branch`
                // unwraps an `if`/`for`/`case` body's `begin…end` and hoists its label, so
                // a `GenItem::Block` only ever arrives here as a free-standing item. Like
                // the region one level up, an UNLABELED one is transparent; a labeled one
                // still mints its scope.
                self.elaborate_gen_scoped(label, items, phase, depth, map, false);
            }

            // ── a plain module-item directly inside generate ─────────
            ast::GenItem::Item(mi) => self.lower_gen_module_item(mi, phase, depth, map),
        }
    }

    /// Elaborate a gen-block body under an OPTIONAL label scope. A `Some(label)`
    /// adds a `label.` prefix segment; an unlabeled body contributes directly to
    /// the current scope (the common LRM behavior when no `begin:label` is given).
    ///
    /// `unlabeled_is_scope` says what an UNLABELED body is for INITIALIZATION ORDER: an
    /// `if`/`for`/`case` body is a generate scope (its content initializes before the
    /// enclosing module's own), while a free-standing `begin…end` in a gen-item list is
    /// only syntax. A labeled body is a scope either way.
    pub(crate) fn elaborate_gen_scoped(
        &mut self,
        label: &Option<ast::Ident>,
        items: &[ast::GenItem],
        phase: GenPhase,
        depth: u32,
        map: &ModuleMap<'_>,
        unlabeled_is_scope: bool,
    ) {
        match label {
            Some(l) => {
                // A generate-if/case/block is a SINGLETON scope — tag it `label[0]`
                // (mirroring generate-for's `label[idx]`) so `is_gen_scope_segment`
                // recognizes it as a GENERATE scope and `walk_scopes` resolves outer
                // nets THROUGH it (a plain `label` would be read as an instance
                // boundary, stopping the outward walk → `t.g.y` undeclared).
                let seg = format!("{}[0]", l.name);
                self.with_scope(&seg, |me| {
                    me.elaborate_generate_scoped(items, phase, depth + 1, map, true);
                });
            }
            None => {
                self.elaborate_generate_scoped(items, phase, depth + 1, map, unlabeled_is_scope)
            }
        }
    }

    /// Lower ONE plain `ModuleItem` found inside a generate, honoring the current
    /// phase. MIRRORS the per-item dispatch in `elaborate_instance` steps
    /// (4)/(7)/(8) — the deliberate reuse the PR calls for.
    pub(crate) fn lower_gen_module_item(
        &mut self,
        mi: &ast::ModuleItem,
        phase: GenPhase,
        depth: u32,
        map: &ModuleMap<'_>,
    ) {
        match (phase, mi) {
            // NETS phase: only net declarations. No ports inside a generate
            // (LRM forbids port decls) → empty port list/body, dir = Internal.
            (GenPhase::Nets, ast::ModuleItem::NetVar(d)) => {
                // A desugared array parameter is created here like any var; its
                // `'{…}` decl-init is collected by the VarInit phase and flushed
                // as an exempt (`lowering_decl_init`) pre-sweep, so it is now a
                // supported form (the old A2a scope-gate that loud-rejected it is
                // lifted). User writes still hit the net-id-keyed const-param deny.
                // `allow_string_init` is TRUE here now. It was false because the decl-time
                // work a string declaration does — a scalar's t0 init, and a fixed string
                // array's `new[n]` pre-size — went straight into the MODULE-scope pending
                // list, so its bare-name lvalue resolved to `t.s` instead of `t.gb[0].s`.
                // Those two pushes are keyed by the declaring scope now
                // (`pending_scoped_presize` / `pending_scoped_bl_strings`) and drained at
                // this scope's own flush, which is what the flag was standing in for.
                self.elaborate_netvar_decl(d, &ast::PortList::None, &[], true);
            }
            // NETS phase: procedural block-local declarations (`initial begin int k; …`).
            // v1 flattens a block-local to a module-scope net, created in the Nets phase so
            // references inside the block resolve; `elaborate_instance` does this for every
            // top-level `module.body` process, and NOT doing it here was a plain scope
            // asymmetry — the identical process one level up worked, while inside a
            // generate scope `int k` was an E3010 at every use. (A generate-scope net
            // DECLARATION already worked; only a decl inside the process did not.)
            //
            // The scope prefix is active, so the net lands as `blk[i].k` and each unrolled
            // iteration gets its own — no cross-iteration coalescing. Ports/body are empty
            // for the same reason the arm above passes them empty (the LRM forbids port
            // decls inside a generate).
            (GenPhase::Nets, ast::ModuleItem::Proc(p)) => {
                self.hoist_block_local_nets(&p.body, &ast::PortList::None, &[]);
            }
            // VARINIT phase: collect this decl's §6.8 pre-sweep initializer (an
            // unpacked-array `'{…}` or a non-constant scalar). For those supported
            // forms the net was created in the Nets phase; `elaborate_generate`
            // flushes the collected inits at scope exit (while the scope prefix is
            // active, so the bare-name lvalue resolves to `blk[idx].name`). Without
            // this the initializer was silently dropped — the value stayed at its
            // X/0 default. (A queue/dyn-array/string decl-init is loud-rejected in
            // the Nets phase — `allow_string_init` is `false` — so no net exists to
            // collect here; that is a documented follow-on.)
            (GenPhase::VarInit, ast::ModuleItem::NetVar(d)) => {
                self.collect_var_init_drivers(d);
            }
            // LOGIC phase: cont-assigns + processes.
            (GenPhase::Logic, ast::ModuleItem::ContAssign(ca)) => {
                self.elaborate_cont_assign(ca);
            }
            // A net-declaration INITIALIZER inside generate (`wire w = a;` /
            // `wire #3 w = a;`) is an implicit continuous assign — lower it as a
            // driver here, exactly like the module-item body path (the net itself
            // was created in the Nets phase). Without this arm the driver was
            // silently dropped (net stuck at z) — a silent-wrong for both the
            // delayed and the plain form.
            (GenPhase::Logic, ast::ModuleItem::NetVar(d)) => {
                self.elaborate_net_init_drivers(d);
            }
            (GenPhase::Logic, ast::ModuleItem::Proc(p)) => {
                let proc = self.lower_proc_block(p);
                debug_assert_eq!(
                    self.processes.len() as u32,
                    self.cur_proc,
                    "ProcId mismatch (generate): fork_modes keyed by cur_proc would miss"
                );
                self.push_process(proc);
            }
            // INSTANCES phase: recurse into child module instances. The parent
            // instance id is `self.cur_inst` (the instance whose body we are in).
            (GenPhase::Instances, ast::ModuleItem::Instance(inst)) => {
                self.elaborate_child_instances(inst, self.cur_inst, map);
            }
            // generate-inside-generate: recurse in the SAME phase, +1 depth.
            (_, ast::ModuleItem::Generate(g)) => {
                // A nested `generate … endgenerate` is another REGION, not a scope.
                self.elaborate_generate_scoped(&g.items, phase, depth + 1, map, false);
            }
            // GAP-G: a `localparam` (or `parameter`) declared INSIDE a generate
            // block (IEEE §27) is a per-instance elaboration constant. Const-fold
            // its value in the current scope (the genvar is bound) and register it
            // under the block-scoped fq key, so a later generate-if condition, a
            // sibling localparam, or a body expression resolves it via
            // `lookup_scoped` — exactly like a module-scope localparam. Registered
            // in EVERY phase because the generate-for re-walks the body per phase
            // and a Logic-phase cont-assign (`stage[g] << R`) needs the binding
            // live when it lowers; the value is identical each pass. A fold
            // failure is reported LOUD once (Nets phase only, to avoid a 4×
            // duplicate). This replaces the old blanket "not allowed inside
            // generate" reject for parameters.
            (_, ast::ModuleItem::Param(p)) => {
                // A REAL-valued parameter has no i64 value, so the integer fold
                // below returns None and the whole declaration went loud — even a
                // bare `localparam real X = 2.5;`. Route it to the real side map
                // first, exactly as the module-scope path does; `param_real_value`
                // applies the §11.8.1 ordering (a real operand puts the expression
                // in the real domain) and hands back an i64 twin only when the
                // initializer was wholly integral, which is what keeps
                // `localparam real R = 4;` usable in integral contexts here too.
                // Idempotent across generate phases, like the integer arm.
                if let Some((rv, exact)) = self.param_real_value(&p.ty, &p.value) {
                    let key = self.fq(&p.name.name);
                    self.real_param_val.insert(key.clone(), rv);
                    if let Some(i) = exact {
                        self.hier_params.insert(key.clone(), i);
                        self.params.insert(key, i);
                    }
                    return;
                }
                // A STRING-valued parameter has no i64 value either — same shape as
                // the real case above, and without this a `localparam string S = "x";`
                // inside a generate block is loud on its own declared default.
                if let Some(raw) = self.param_str_or_folded(p, false) {
                    let key = self.fq(&p.name.name);
                    self.str_param_raw.insert(key, raw);
                    return;
                }
                // A generate-scope parameter has no override channel, so its
                // declared default is always what binds.
                let meta = self.param_decl_width_unoverridden(p);
                match self.const_eval_in_scope(&p.value) {
                    Some(v) => {
                        // The width the caller just resolved, not a re-derivation:
                        // this scope has no override channel, so `meta` is authoritative.
                        let v = self.coerce_param_value_with(v, meta);
                        let key = self.fq(&p.name.name);
                        self.hier_params.insert(key.clone(), v);
                        // ⚠️ The declared width/sign and range were NOT recorded here,
                        // while both the module-body and header paths record them. So a
                        // generate-scope `localparam logic [1:0] M = 2'b01;` read back
                        // as 32 bits — `$display("%b", M)` printed thirty leading zeros
                        // where iverilog prints `01`, and the same width feeds concats
                        // and comparisons. Value right, width silently wrong.
                        if let Some(m) = meta {
                            self.param_meta.insert(key.clone(), m);
                        }
                        if let Some(r) = self.param_decl_range(p) {
                            self.param_range.insert(key.clone(), r);
                        }
                        self.params.insert(key, v);
                    }
                    None => {
                        // Wider than the i64 domain — see `wide_param_bits`. Reached
                        // only after the fold declined, so a wide declaration whose
                        // value fits keeps its integer identity.
                        let wide = meta
                            .and_then(|(w, sg)| self.wide_param_const_in_scope(&p.value, w, sg));
                        if let Some(cv) = wide {
                            let key = self.fq(&p.name.name);
                            self.wide_param_bits.insert(key, cv);
                            return;
                        }
                        if phase == GenPhase::Nets {
                            self.param_value_unfoldable(
                                "generate-scope parameter",
                                &p.name.name,
                                &p.value,
                            );
                        }
                    }
                }
            }
            // A PORT declaration inside generate stays forbidden (IEEE §27:
            // ports are module-boundary, not per-instance). Reported once.
            (GenPhase::Nets, ast::ModuleItem::PortDecl(_)) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "port declaration not allowed inside generate",
                );
            }
            (
                GenPhase::Nets,
                ast::ModuleItem::Func(_) | ast::ModuleItem::Task(_) | ast::ModuleItem::Defparam(_),
            ) => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "construct deferred inside generate (func/task/defparam)",
                );
            }
            // Genvar decl inside generate: elaboration-only, no net → no-op.
            // Any item not matching the active phase: no-op (handled elsewhere).
            _ => {}
        }
    }
    /// Collect `sequence`/`property` declarations from a generate block into the
    /// module-global tables (slice A4). Walks the generate STRUCTURE — For/If/Case/
    /// Block bodies and nested generates — WITHOUT const-eval or genvar binding: a
    /// declaration is a definition (registered once), not per-instance logic.
    /// Limitations (loud, never silent-wrong): both branches of a generate-if are
    /// collected (a same-named decl in each yields a benign redeclare warning,
    /// first-wins); a genvar-parameterized decl body keeps its unbound genvar, which
    /// is a loud unresolved-name at the use site.
    pub(crate) fn collect_gen_sva_decls(&mut self, items: &[ast::GenItem]) {
        for gi in items {
            match gi {
                ast::GenItem::Item(boxed) => match &**boxed {
                    ast::ModuleItem::SequenceDecl(s) => self.register_seq_decl(s),
                    ast::ModuleItem::PropertyDecl(p) => self.register_prop_decl(p),
                    ast::ModuleItem::Generate(g) => self.collect_gen_sva_decls(&g.items),
                    _ => {}
                },
                ast::GenItem::For { body, .. } => self.collect_gen_sva_decls(body),
                ast::GenItem::Block { items, .. } => self.collect_gen_sva_decls(items),
                ast::GenItem::If { then_b, else_b, .. } => {
                    self.collect_gen_sva_decls(then_b);
                    self.collect_gen_sva_decls(else_b);
                }
                ast::GenItem::Case { items, .. } => {
                    for it in items {
                        match it {
                            ast::GenCaseItem::Match { body, .. }
                            | ast::GenCaseItem::Default { body, .. } => {
                                self.collect_gen_sva_decls(body)
                            }
                        }
                    }
                }
            }
        }
    }
}
