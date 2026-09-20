//! Interface INSTANCE elaboration (`ifc u();` — params, member nets, the scope's own
//! §6.8 pre-sweep, then its logic) — split out of `ports.rs` (mechanical move;
//! module-size policy).

use super::*;

impl Elaborator<'_> {
    pub(crate) fn elaborate_iface_instances(&mut self, mi: &ast::ModuleInstance, wire_phase: bool) {
        let iface_name = mi.module_name.name.clone();
        let Some(decl) = self.ifaces.get(&iface_name).cloned() else {
            return;
        };
        // v6 ②: per-instance parameter overrides — resolved NOW, in the
        // PARENT scope, exactly like the module-instance path (Fix 1).
        let mut overrides: Vec<ResolvedOverride> = Vec::with_capacity(mi.param_overrides.len());
        for ov in &mi.param_overrides {
            match ov {
                ast::ParamConn::Positional(e) => {
                    let value = self.const_eval_in_scope(e);
                    // Build the record BEFORE deciding what to say about it: the
                    // other two channels are computed from the same `e`, and the
                    // warning below is a statement about the record.
                    let ovr = ResolvedOverride {
                        name: None,
                        value,
                        is_named: false,
                        had_value: true,
                        fill: expr_as_fill(e).map(|(k, r)| (k, r.to_string())),
                        str_is_literal: Self::param_str_literal(e).is_some(),
                        str: self.const_str_in_scope(e),
                        bits: self.override_bits(e),
                        signed: Some(self.const_signed_env(e, &ConstWidths::new())),
                        self_meta: self.override_self_meta(e),
                        self_val: self
                            .override_self_meta(e)
                            .and_then(|m| self.override_self_value(e, m)),
                        array: None,
                        elem_select: false,
                    };
                    if value.is_none() {
                        if Self::expr_is_real_literal(e) {
                            self.error(
                                MsgCode::ElabUnsupported,
                                "overriding a parameter with a real value is unsupported \
                                 (a real override cannot be folded); the default would \
                                 be used silently",
                            );
                        } else if self.count_reads_real_param(e) {
                            // r19/B1: the guard next to this one tests a real LITERAL, but this
                            // slice newly made real-VALUED expressions reachable here (an ident
                            // bound to a real param, `R+1`, `R*2`). Those do not const-fold, so
                            // they fell into the warn-and-keep-default path below = the child
                            // silently ran with the WRONG parameter, changing port widths at
                            // exit 0 where this was loud before the slice.
                            self.error(
                                MsgCode::ElabUnsupported,
                                "a parameter override that reads a real parameter is unsupported \
                         (a real has no integral constant value)",
                            );
                        } else if ovr.keeps_default() {
                            self.warn(
                                "parameter override expression is not a constant; default kept",
                            );
                        }
                    }
                    overrides.push(ovr);
                }
                ast::ParamConn::Named { name, value, .. } => {
                    // Same shape as the positional arm: the two non-i64
                    // channels are decided from `value` alone, so compute them
                    // first and let the warning ask the record.
                    let fill = value
                        .as_ref()
                        .and_then(|e| expr_as_fill(e).map(|(k, r)| (k, r.to_string())));
                    let text_is_literal =
                        value.as_ref().and_then(Self::param_str_literal).is_some();
                    let text = value.as_ref().and_then(|e| self.const_str_in_scope(e));
                    let v = value.as_ref().and_then(|e| {
                        let r = self.const_eval_in_scope(e);
                        if r.is_none() {
                            // r19: a REAL-literal override is ERROR, not warn-and-keep.
                            // Real PARAMETERS are supported now, so silently running with
                            // the declared default would be the wrong value with exit 0 —
                            // and the override machinery is i64-only, so the right value
                            // cannot be applied. Every other non-constant override keeps
                            // the pre-existing warn-and-default behaviour.
                            if Self::expr_is_real_literal(e) {
                                self.error(
                                    MsgCode::ElabUnsupported,
                                    &format!(
                                        "overriding parameter `{}` with a real value is \
                                         unsupported (a real override cannot be folded); \
                                         the declared default would be used silently",
                                        name.name
                                    ),
                                );
                            } else if ResolvedOverride::keeps_default_of(
                                None,
                                fill.as_ref(),
                                text.as_ref(),
                                self.override_bits(e).as_ref(),
                            ) {
                                self.warn(&format!(
                                    "override of parameter `{}` is not a constant; default kept",
                                    name.name
                                ));
                            }
                        }
                        r
                    });
                    overrides.push(ResolvedOverride {
                        name: Some(name.name.clone()),
                        value: v,
                        is_named: true,
                        had_value: value.is_some(),
                        fill,
                        str_is_literal: text_is_literal,
                        str: text,
                        bits: value.as_ref().and_then(|e| self.override_bits(e)),
                        signed: value
                            .as_ref()
                            .map(|e| self.const_signed_env(e, &ConstWidths::new())),
                        self_meta: value.as_ref().and_then(|e| self.override_self_meta(e)),
                        self_val: value.as_ref().and_then(|e| {
                            self.override_self_meta(e)
                                .and_then(|m| self.override_self_value(e, m))
                        }),
                        array: None,
                        elem_select: false,
                    });
                }
            }
        }
        let ports_ok = match &decl.ports {
            ast::PortList::None => true,
            ast::PortList::Ansi(v) => v.iter().all(|p| p.iface.is_none()),
            ast::PortList::NonAnsi(v) => v.is_empty(),
        };
        if !ports_ok {
            self.error(
                MsgCode::ElabUnsupported,
                "non-ANSI or interface-typed header ports on an interface are outside the MVP",
            );
            return;
        }
        for item in &mi.instances {
            if !item.unpacked.is_empty() {
                self.error(
                    MsgCode::ElabUnsupported,
                    "interface instance arrays are outside the MVP",
                );
                continue;
            }
            let path = self.child_prefix(&item.name.name);
            if !self.iface_insts.contains_key(&path) {
                let saved_prefix = std::mem::replace(&mut self.cur_prefix, path.clone());
                // params (header `#(...)` then body localparams) BEFORE nets
                // so `[W-1:0]` folds — mirroring module passes (3)/(3b).
                // §3 ⑤ ⓕ: the interface's imports. Two passes around `bind_params`,
                // exactly as the module scope does: a compilation-unit or HEADER
                // import (`interface i import p::*; #(parameter N = W)`) is visible to
                // the header's own defaults, a body import only after.
                //
                // §3.b `iface-pkg-routine`: CONSTANTS were once the only thing bound
                // here, on the reasoning that "an interface body has no
                // functions/tasks, so a routine brought in by an import has no caller
                // to resolve". The premise was about DECLARATIONS and said nothing
                // about CALLS, which an interface body writes freely and both oracles
                // run. Routines and constant functions bind below too — and since row
                // `iface-subr` the premise is false outright: an interface body
                // DECLARES functions and tasks like a module body, so the local
                // definitions are registered below as well and are what a wildcard
                // import loses to.
                let iface_imports: Vec<ast::ImportDecl> = self
                    .cu_imports
                    .clone()
                    .into_iter()
                    .chain(decl.body.iter().filter_map(|it| match it {
                        ast::ModuleItem::Import(i) => Some(i.clone()),
                        _ => None,
                    }))
                    .collect();
                let n_cu = self.cu_imports.len();
                // §3.b: from here to the window exit the ROUTINE scope is this
                // interface instance's own, not the parent module's — see
                // [`RoutineScope`] for the two measured defects the parent's tables
                // being live caused. `inst_stack` is pushed with it because
                // `seed_subroutine_routes` / `note_subroutine_route`
                // (`frames_body.rs`) key the OBS `subroutines[]` rows on
                // `inst_stack.last()`: without the push an interface's routines would
                // be filed under the parent module's name.
                let saved_rtn = self.take_routine_scope(
                    path.clone(),
                    iface_name.clone(),
                    iface_imports.clone(),
                );
                self.inst_stack.push(iface_name.clone());
                let local_names = self.gather_local_decl_names(&decl);
                // The five block-local classifier maps, computed from THIS INTERFACE and
                // held across both the Nets pass and the Logic loop below — the module
                // path's `instance.rs:565-586`, verbatim, with `module` -> `&decl`.
                //
                // No signature work: `hdl_ast::Item::Interface` holds an
                // `ast::ModuleDecl`, so every one of these already accepts an interface
                // body; they were simply never called with one. Until now the maps in
                // scope here described the PARENT module (this runs inside the parent's
                // Nets phase), so the hoist below had to refuse any body containing a
                // user-written block-local — see the deleted admission predicate.
                //
                // ⚠️ All five or none. `block_local/hoist.rs`'s `shadows_module` is the
                // only term that routes a member-colliding local to its own `$blk$<lo>`
                // net, and it reads `self.local_decl_names`; installing four of the five
                // would admit more bodies to the hoist while still resolving the
                // collision against the parent's names.
                let mut wc_origin: BTreeMap<String, String> = BTreeMap::new();
                let mut explicit_imports: std::collections::BTreeSet<String> =
                    std::collections::BTreeSet::new();
                let mut saved_params: Vec<(String, Option<i64>)> = Vec::new();
                // §3.b: the constant-interpreter half of the routine import, mirroring
                // `instance.rs`'s (3a.5). Without it a `localparam W = g(40)` in an
                // interface body was `E3009 … has no constant-fold arm` where both
                // oracles fold it (census c21; the module twin m21 already folded).
                //
                // §3.b `iface-subr`: and the interface's OWN functions join
                // `const_func_table` here too, the other half of (3a.5) — BEFORE
                // `bind_params` below, so a header default `#(parameter int X = cf(4))`
                // folds through a body-declared `cf` (census d10b, both oracles `X=44`)
                // as well as a body `localparam int W = cf(4)` (d10, `W=44`). That also
                // makes `local_const_funcs` a real set: a declared constant function
                // wins a wildcard import of the same name, §26.3 (d23, `W=44` — the
                // interface's own `f` at `*11`, not `pk::f` at `+4`).
                let mut local_const_funcs: BTreeSet<String> = BTreeSet::new();
                for it in &decl.body {
                    if let ast::ModuleItem::Func(f) = it {
                        self.const_func_table.insert(f.name.name.clone(), f.clone());
                        local_const_funcs.insert(f.name.name.clone());
                    }
                }
                let mut wc_const_fn: BTreeMap<String, String> = BTreeMap::new();
                let mut explicit_const_fn: BTreeSet<String> = BTreeSet::new();
                for (i, imp) in iface_imports.iter().enumerate() {
                    if Self::import_precedes_header(&decl, n_cu, i, imp) {
                        self.apply_import_consts(
                            imp,
                            &mut saved_params,
                            &mut wc_origin,
                            &mut explicit_imports,
                            &local_names,
                            i >= n_cu,
                        );
                        self.apply_import_const_funcs(
                            imp,
                            &local_const_funcs,
                            &mut wc_const_fn,
                            &mut explicit_const_fn,
                            i < n_cu,
                        );
                    }
                }
                let param_ovr = {
                    let (sp, ovr) = self.bind_params(&decl, &overrides);
                    saved_params.extend(sp);
                    ovr
                };
                for (i, imp) in iface_imports.iter().enumerate() {
                    if !Self::import_precedes_header(&decl, n_cu, i, imp) {
                        self.apply_import_consts(
                            imp,
                            &mut saved_params,
                            &mut wc_origin,
                            &mut explicit_imports,
                            &local_names,
                            i >= n_cu,
                        );
                        self.apply_import_const_funcs(
                            imp,
                            &local_const_funcs,
                            &mut wc_const_fn,
                            &mut explicit_const_fn,
                            i < n_cu,
                        );
                    }
                }
                // §3.b `iface-subr`: the interface's OWN `function`/`task`
                // declarations, the module lane's step (3.5), through the ONE
                // registration both lanes share (`rtn_decl.rs`). BEFORE the import
                // loop below, because that loop is skip-if-present: a declaration
                // already in the table is what makes a local definition win a
                // wildcard import of the same name (§26.3 — census d02, both oracles
                // `R=1040`, the interface's own `g` at `+1000` rather than `pk::g` at
                // `+4`).
                //
                // The containment gate each body gets in the module lane's (3.5) is
                // NOT here: it runs below, in the same loop that gates the `Proc`
                // bodies, because in THIS lane the five block-local classifier maps
                // are installed after the import passes. Gating at registration time
                // would ask the PARENT module's maps.
                for it in &decl.body {
                    self.register_declared_routine(it);
                }
                // §3.b `iface-subr` (round-2 soundness S-4): a MODPORT and a
                // `function`/`task` of an interface share one name space, so
                // `function int mp(…)` beside `modport mp (…)` is an illegal design —
                // iverilog "'mp' has already been declared in this scope", verilator
                // "MODPORT 'mp' has the same name as function: 'mp'". Both namespaces
                // only became live together in this window with this row, and nothing
                // compared them: POST ran the design and printed `R=44`.
                //
                // HERE, before the import loop, so the table holds exactly this
                // interface's OWN declarations: an imported routine is a different
                // question (`apply_import_routines` owns the import-vs-declaration
                // collision) and must not be reported as a modport clash. `rtn_pkg` is
                // asked anyway, as the same discriminator the import guard uses, so the
                // check keeps its meaning if it is ever moved.
                for it in &decl.body {
                    if let ast::ModuleItem::Modport(mp) = it {
                        let n = &mp.name.name;
                        if (self.func_table.contains_key(n) || self.task_table.contains_key(n))
                            && !self.rtn_pkg.contains_key(n)
                        {
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!(
                                    "modport `{n}` has the same name as a function/task \
                                     declared in this interface"
                                ),
                            );
                        }
                    }
                }
                // §3.b: the RUNTIME half — imported functions/tasks join this
                // interface's own tables under the same §26.3 rules the module
                // lane's (3.6) applies: an explicit import always wins, a local
                // definition wins a wildcard, and one name from two different
                // wildcard imports is ambiguous, hence unbound and loud at the use
                // site.
                let mut wc_rtn: BTreeMap<String, String> = BTreeMap::new();
                let mut explicit_rtn: BTreeSet<String> = BTreeSet::new();
                // Round-3 R2-1: `i < n_cu` — `iface_imports` is `cu_imports` chained
                // in front of this interface's own imports, exactly as the module
                // lane's `import_list` is, so the same index separates a `$unit`
                // import (which a local declaration SHADOWS, §26.4) from one written
                // in this interface (which it COLLIDES with, §26.3).
                for (i, imp) in iface_imports.iter().enumerate() {
                    self.apply_import_routines(imp, &mut wc_rtn, &mut explicit_rtn, i < n_cu);
                }
                // §2 Scoping row 3 (round-1 delta): the five block-local classifier maps
                // are computed HERE, after BOTH `apply_import_consts` passes above
                // (header at the loop before `bind_params`, body at the loop just above)
                // and before the body-parameter fold — the module lane's proven order
                // (`instance.rs`: header imports :511, body imports :541, maps :590-610).
                // They used to be computed before the imports, which made
                // `names_with_pkg_var_aliases` a guaranteed no-op here: `pkg_var_aliases`
                // held no entry for this interface scope yet, so an interface
                // block-local colliding with an imported package variable still
                // flattened onto the package net (measured: `pkgv=100` where both
                // oracles say 7, and the same cross-instance and continuous-assign
                // leaks the module lane had).
                //
                // The WHOLE computation moved, not just the augmentation: between the
                // old site and here, `compute_coalesced_block_locals` consumes
                // `self.scoped_block_locals` and the `check_block_local_scope_leaks`
                // gate reads it too, so splitting them would have fed the gate a map
                // built from a different name set than the hoist later reads.
                // §3.b `iface-subr` (round-3 soundness R2-2): the IEEE 1800 §6.10
                // use-before-declaration tables. `check_decl_precedes_use`
                // (`scope.rs`) is keyed on all three of these, and the window never
                // installed any of them, so the gate was VACUOUS for every body
                // lowered inside it: `decl_pos` held the PARENT module's positions,
                // and `decl_pos_range` — the parent's span — excluded the interface's
                // own text, so the gate returned at its second line for every name in
                // it. Measured: a declared function reading a net declared BELOW it
                // (`int r = lp(3);` with `int later = 100;` after) printed `R=3 L=100`
                // at exit 0, a value NEITHER oracle produces (iverilog rejects
                // "Check for declaration after use", verilator `R=303`), while the
                // MODULE twin is loud on PRE and POST alike.
                //
                // POSITION mirrors the module lane's (3b) (`instance.rs`): after both
                // import passes and `bind_params`, immediately before the block-local
                // maps, so the interface's body-parameter fold, its routine bodies and
                // its processes are all gated — and so that `decl_block_locals`, the
                // gate's own block-local exemption, is installed in the same breath.
                // Nothing between the two lines below resolves a name.
                let dpos = self.gather_decl_positions(&decl);
                let saved_decl_pos = std::mem::replace(&mut self.decl_pos, dpos);
                let saved_dpos_scope = std::mem::replace(&mut self.decl_pos_scope, path.clone());
                let saved_dpos_range =
                    std::mem::replace(&mut self.decl_pos_range, (decl.span.lo, decl.span.hi));
                let dbl = self.gather_block_local_names(&decl);
                let saved_dbl = std::mem::replace(&mut self.decl_block_locals, dbl);
                // §2 Scoping row 3: same augmented SCOPING feed as `instance.rs` (see
                // `names_with_pkg_var_aliases`). Only the scoped set gets it —
                // `compute_per_entry_block_locals` below reads `module_names` with the
                // opposite polarity, and `local_names` itself stays untouched.
                let shadow_names = self.names_with_pkg_var_aliases(&local_names);
                // §2 Scoping queue row 1: the gather is kept for the same reason the
                // module lane keeps it — a `pk::g()` scoped call inside this interface
                // body reserves its frame on demand and feeds this gather then
                // (`feed_scoped_block_locals`). Measured: census c17, `Z=88` at HEAD
                // where both oracles say 44.
                //
                // §3.b: the bodies the routine import above just bound are fed here
                // too, the module lane's step (3.6a) with the same shared collector.
                // They are not in `decl.body`, so without the feed a package routine's
                // two same-named sibling block-locals flatten onto ONE net. Computed
                // in one call rather than the module lane's compute-then-redo, because
                // the imports are already applied at this point in this lane. With no
                // package routine bound, `refs` is empty and this is the previous
                // `&[]` call, `scoped_gather_fed` included.
                let extra = self.imported_routine_bodies();
                let refs: Vec<&ast::Stmt> = extra.iter().collect();
                let (scoped_blocks, scoped_gather) =
                    Self::compute_scoped_block_locals(&decl, &shadow_names, &refs);
                let saved_scoped_blocks =
                    std::mem::replace(&mut self.scoped_block_locals, scoped_blocks);
                let saved_scoped_gather = std::mem::replace(&mut self.scoped_gather, scoped_gather);
                let saved_scoped_fed = std::mem::replace(
                    &mut self.scoped_gather_fed,
                    refs.iter().map(|b| Self::stmt_span_key(b)).collect(),
                );
                let per_entry_blocks = Self::compute_per_entry_block_locals(&decl, &local_names);
                let saved_per_entry_blocks =
                    std::mem::replace(&mut self.per_entry_block_locals, per_entry_blocks);
                let coalesced = Self::compute_coalesced_block_locals(
                    &decl,
                    &local_names,
                    &self.scoped_block_locals.clone(),
                );
                let saved_coalesced =
                    std::mem::replace(&mut self.coalesced_block_locals, coalesced);
                let saved_local_names =
                    std::mem::replace(&mut self.local_decl_names, local_names.clone());
                // ⚠️ The maps are not the only thing the module path does before it
                // hoists. `instance.rs:801` also runs the CONTAINMENT gate over every
                // process body, and admitting the interface body to the hoist without it
                // is loud→silent-wrong — the direction the accuracy ladder forbids:
                //
                //   interface ifb; initial begin : outer int x; x = 1;
                //     begin : inner int x; x = 2; o1 = x; end
                //     #1 o2 = x;        // must read the OUTER x
                //   end endinterface
                //
                // measured PRE `E3010` ×8 (refused), POST-without-this-call
                // `o1=02 o2=02` at exit 0 where both oracles give `o1=02 o2=01`, while
                // the MODULE twin of the identical body still emits the E3009 in POST.
                // The flat per-body block-local table is what the gate is about and the
                // interface body has the same one, so it needs the same gate.
                //
                // It must run INSIDE the map window: the gate consults
                // `scoped_block_locals` to skip a name that owns a `$blk$<lo>` net, and
                // outside the window that map is the parent module's.
                //
                // §3.b `iface-subr`: the interface's OWN routine bodies are gated in
                // THIS loop, not where they are registered above. The module lane
                // gates them inside its step (3.5) because its maps are already
                // installed by then (`instance.rs` (3b) precedes (3.5)); here (3.5)
                // had to move ahead of the import loop, which is ahead of the maps.
                // One loop in declaration order keeps the diagnostic ORDER the module
                // twin emits. Measured: d11 (an outer block-local read after an inner
                // same-named declaration, inside a declared function) stays loud with
                // the module twin m11's message, while d01's `bl` — a block-local that
                // never leaves its block — passes.
                for it in &decl.body {
                    match it {
                        ast::ModuleItem::Proc(p) => self.check_block_local_scope_leaks(&p.body),
                        ast::ModuleItem::Func(f) => self.check_block_local_scope_leaks(&f.body),
                        ast::ModuleItem::Task(t) => self.check_block_local_scope_leaks(&t.body),
                        _ => {}
                    }
                }
                // §3.b: the same gate on the imported routine bodies, AFTER the maps
                // above are installed, so a declaration that now owns a `$blk$<lo>`
                // net is exempt exactly as it is for a scope-declared routine. The
                // module lane runs it in (3.6a) for the same reason.
                for body in &extra {
                    self.check_block_local_scope_leaks(body);
                }
                for it in &decl.body {
                    if let ast::ModuleItem::Param(pp) = it {
                        // Same binder as the module body loop and the generate fold.
                        // This loop used to be its own reduced copy — `const_eval` →
                        // `coerce` → `hier_params`/`params`, and NOTHING else — so an
                        // interface body parameter never recorded `param_meta` (its
                        // declared width and sign) or `param_range`, and a `string` /
                        // `real` / >64-bit one was not routed at all (`parameter S =
                        // "abc"` in an interface was loud E3009 on its own default).
                        // `generate.rs` carries a ⚠️ note about repairing exactly this
                        // in the generate fold; the interface was the last REDUCED copy.
                        //
                        // THREE full copies remain — `instance.rs`'s module-body fold,
                        // `generate.rs`'s, and `package.rs`'s — and they disagree with
                        // this binder in more than one place, measured: `generate.rs`
                        // and `package.rs` fold with `const_eval_in_scope` instead of
                        // `eval_param_init`, so a fill DEFAULT is sized to 32 bits and
                        // not to the declared width (`parameter [63:0] Q = '1` reads
                        // `0000_0000_ffff_ffff` in both), and `package.rs` records no
                        // `param_range` (a package `parameter [15:8] P` part-selects to
                        // `x`) and routes neither `string` nor `real`. All of that is
                        // pre-existing and identical in PRE — a separate slice, one
                        // line in ROADMAP §3. Do NOT fix an instance of it here: this
                        // is a class, and the funnel is this function.
                        //
                        // With no ANSI header these declarations ARE the overridable
                        // parameters (`param_ports`), so binding them here also applies
                        // an override that targets one. Binding ONLY those through the
                        // shared path and leaving the rest on the reduced copy is what
                        // the first cut did, and it made a parameter's registered WIDTH
                        // depend on whether it happened to be overridden: two instances
                        // of one interface then disagreed inside a single run at exit 0
                        // (`ifc #(.P(8'hA5)) a(); ifc b();` — same value, `a.P[15:12]=a`
                        // and `b.P[15:12]=0`). One spelling for every declaration.
                        self.bind_one_param(pp, &param_ovr, &mut saved_params);
                        // The i64 twin, republished so `i0.P` stays readable from
                        // outside. This is PRE's behaviour and the measured reason to
                        // keep it is arithmetic, not sentiment: over 13 consumers × 6
                        // exact-integer values, the i64 view is CORRECT in 72 cells and
                        // wrong in 6 — every wrong cell is `/` with a fractional
                        // quotient (`P=5` → `i0.P/2` gives 2.0, iverilog 2.5). Dropping
                        // it took `int'`, `$rtoi`, `$sqrt`, `*1.5`, `+0.5`, `>`, a real
                        // assignment and the bare read from correct to loud — 72
                        // correct→loud regressions to remove 6 silent-wrong ones. (An
                        // earlier revision of this comment claimed the reverse and cited
                        // `P = 4`; the discriminator is not the VALUE but the OPERATOR —
                        // only division with a fractional quotient separates the two
                        // domains, so `P = 8` is correct at `/2` as well.)
                        //
                        // ⚠️ It does leave one declaration answering two ways in a
                        // single run: the BARE read now reaches `real_param_val` through
                        // the binder above and is 2.5, while this hierarchical twin is
                        // 2.0. That split is the honest state of the hierarchical-real
                        // axis, not a property of this line — patching the deferred
                        // placeholder with a real constant instead breaks strictly more
                        // cells (every integral consumer reads the IEEE-754 bits).
                        // ROADMAP §2 owns it.
                        if let Some((_, Some(i))) = self.param_real_value(&pp.ty, &pp.value) {
                            let key = self.fq(&pp.name.name);
                            self.hier_params.insert(key, i);
                        }
                    }
                }
                // ANSI header ports → nets (the iface body + `i.<port>` see them).
                self.elaborate_ports(&decl.ports);
                // nets first (declaration order), then logic — mirroring the
                // module body passes (4)/(7).
                // §4.5.265: the net-decl loop runs INSIDE the instance's rank scope too,
                // because a declaration records its pre-size and its block-local
                // initializers under the rank path in effect at the DECLARATION — and the
                // flush below claims by that path. Creating the nets outside the scope and
                // flushing inside it meant no flush ever claimed them, which the
                // never-emitted guard reported (loudly, which is the point of it).
                // The PARENT module instance's path — the key of the sibling-to-sibling
                // [`StaticScopedCarry`]. Taken here because step 6.5 and its adopt run
                // inside the closure below, and `saved_rtn` cannot be borrowed there.
                let parent_inst = saved_rtn.inst_prefix.clone();
                let slot = self.rank_slot_for_instance();
                let rkey = (self.rank_band, item.name.span.lo, 0);
                self.with_rank_scope_keyed(slot, rkey, |sc| {
                    for it in &decl.body {
                        if let ast::ModuleItem::NetVar(d) = it {
                            // A desugared array parameter is created like any var; its
                            // `'{…}` decl-init rides the interface §6.8 pre-sweep below
                            // (collect + flush, `lowering_decl_init`-exempt), so it is a
                            // supported form now (the A2a scope-gate is lifted). User
                            // writes still hit the net-id-keyed const-param deny.
                            // `allow_string_init` is TRUE here now, for the same reason as the
                            // generate walk: the flag was standing in for a string
                            // declaration's decl-time writes landing in the MODULE-scope
                            // pending list, where the bare-name lvalue resolved outside this
                            // instance's prefix. Those are keyed by the declaring scope now.
                            sc.elaborate_netvar_decl(d, &decl.ports, &decl.body, true);
                        }
                    }
                    // §6.8 pre-sweep for the interface body (mirrors the module-body
                    // sweep): an array `'{…}` / non-constant decl-init has no foldable
                    // `net.init`, so without this collect+flush it was silently dropped.
                    // Runs in the interface INSTANCE scope (bare-name lvalues resolve to
                    // `path.name`) and BEFORE the logic pass below, so the synthesized
                    // `initial` precedes the interface's own procs.
                    //
                    // SAVE/RESTORE the shared `pending_var_inits` around it: this pass
                    // runs during the PARENT's Nets phase, and `hoist_block_local_nets`
                    // may already have queued a module block-local non-const init there
                    // (it runs earlier, in pass 4a). Without the isolation this flush
                    // would STEAL that init and re-lower it in the interface scope —
                    // both a loud misresolve and (with same-named members) a silent
                    // module-side drop. The generate VarInit walk isolates the same way.
                    // §4.5.259: an interface instance is a SCOPE of its own, so it takes the
                    // instance slot like a module child. Without a scope of its own its flush
                    // borrowed the ENCLOSING scope's own-variables slot, and — because its two
                    // call sites run in different passes than the module's own flush — the
                    // rank vectors collided outright: a module's own initializer ran BETWEEN
                    // two interfaces, and a generate-nested interface ran after the generate's
                    // own variable. Both are the enclosing scope's slot, decided by tie-break.
                    let saved_pending = std::mem::take(&mut sc.pending_var_inits);
                    // Partial branch parity with `instance.rs`'s module-body pass: a
                    // procedural block-local flattens to a scope-level net, and the
                    // interface body never did it at all — so the `__foreach_<i>_<n>` /
                    // `__foreach_st_<n>` pair the PARSER synthesizes for every `foreach`
                    // resolved to nothing. Measured: an identical body is correct in a
                    // `module` and emits 9 errors in an `interface`, seven of them
                    // `undeclared net/variable top.u.__foreach_i_<n>` and two the actively
                    // misleading "enum method `v.first` is unavailable" on a design with
                    // no enum. Both oracles run it.
                    //
                    // ⚠️⚠️ ADMITTED ONLY WHEN EVERY block-local reachable from this
                    // interface's procs is one of those SYNTHESIZED names. The module
                    // twin does not just call the hoist — `instance.rs` first computes
                    // `local_decl_names`, `decl_block_locals`, `scoped_block_locals`,
                    // `per_entry_block_locals` and `coalesced_block_locals` from the
                    // MODULE, and those are what keep a block-local that collides with a
                    // scope-level name from coalescing onto it. Here those maps hold the
                    // PARENT module's names (this pass runs inside the parent's Nets
                    // phase), so the interface's own members are invisible to them.
                    // Measured with the hoist ungated: `interface ifc; integer b; …
                    // begin integer b; b = 7; end` printed `OUTER b=7` and `u.b = 7`
                    // where both oracles print 99 — while the MODULE twin of the same
                    // text is correct in PRE and POST. That is a loud→silent-wrong, so
                    // the general case stays refused; its prerequisite is those five
                    // passes taught to run over an interface body (they are all
                    // `&ast::ModuleDecl`-typed today) and held across BOTH this Nets pass
                    // and the Logic pass below. ROADMAP §3 carries it.
                    //
                    // The synthesized names need none of that: each embeds its own
                    // `foreach` token offset, so no two can collide and none can be
                    // written by a user — proved per design by the `local_names` check
                    // rather than assumed from the prefix.
                    //
                    // ⚠️ INSIDE the isolation, not before it. The hoist queues a
                    // block-local's non-constant decl-init into the shared
                    // `pending_var_inits`, and this pass runs during the PARENT's Nets
                    // phase — queued outside the `take` those inits would ride the
                    // MODULE's pending list and be lowered against the module prefix,
                    // which is the misresolve the save/restore above exists to prevent.
                    for it in &decl.body {
                        if let ast::ModuleItem::Proc(p) = it {
                            sc.hoist_block_local_nets(&p.body, &decl.ports, &decl.body);
                        }
                    }
                    // §3.b: step (6.5)'s function, at the same position relative to
                    // this scope's nets as the module lane runs it — after the nets
                    // exist (so a frame net lands outside them) and after the hoist,
                    // and BEFORE the decl-initializer collection below and the Logic
                    // loop, so both a `int r = lp(5);` and a call site there can divert
                    // to a reserved FuncId. It classifies the tables the routine import
                    // filled (`build_frame_set` / `build_task_frame_set`) and reserves +
                    // lowers what needs a frame: a loop body (c14), a task with a delay
                    // (c15), an output formal (c3, c20). No-op when both sets are empty.
                    //
                    // §3.b `iface-subr`: those tables now also hold the interface's OWN
                    // declarations, so this call is what frames a declared recursive
                    // function (d08 `FACT=120`), a declared task with a delay (d06
                    // `D=21 @1`), a declared loop body (d07 `LP=10`) and an output
                    // formal (d01 `T=103`). A STATIC declared routine's bare-name key
                    // has no `::` in it, so `static_scoped_keys` never carries it to a
                    // sibling instance and its local stays per instance — which is what
                    // both oracles measure for a routine declared in the interface (d05
                    // `L1=10 L2=10`, and the module twin m05 identical), unlike a
                    // PACKAGE routine's static local (ROADMAP §2).
                    //
                    // ⚠️ POSITION, round-2 soundness S-5: this used to run BELOW the
                    // whole rank-scope block, i.e. after `collect_var_init_drivers`. A
                    // net DECL-INITIALIZER calling a FRAMED declared routine
                    // (`int r = lp(5);` where `lp` has a loop) was then collected with
                    // no frame reserved, routed to the CONSTANT interpreter and refused
                    // as `E3009 function \`lp\` body is not reducible to an expression`
                    // — where the MODULE twin prints `R=10` and so do both oracles. The
                    // module lane's order is nets (4) → hoist (4a) → 6.5 → (6.9)
                    // `collect_var_init_drivers` → flush; this is that order.
                    //
                    // INSIDE the `pending_var_inits` isolation, for the module lane's
                    // reason: whatever a frame reservation queues there belongs to THIS
                    // scope's flush below, exactly as the module lane's single shared
                    // list gives it to the module's own flush.
                    //
                    // The frame nets land inside the PARENT Instance's
                    // `[first_net, net_count)` slice. That is not a new property of this
                    // call: `net_count` has no consumer outside elaborate's own tests,
                    // and the scoped `pk::g()` lane (census c18, correct before this
                    // slice) already reserves frames from inside this same window.
                    sc.lower_frame_funcs();
                    // §3.b, round-2 delta: the static scoped frames join the window
                    // HERE, between step 6.5 and the Logic loop that makes the scoped
                    // calls — see [`Self::adopt_static_scoped_frames`] for why neither
                    // side of that boundary works. It travels WITH step 6.5, because
                    // the reason it must follow that call (6.5 re-reserves every
                    // `func_table` entry and clears the three call-shape sets) is a
                    // property of the call, not of the position in the window.
                    sc.adopt_static_scoped_frames(&parent_inst);
                    for it in &decl.body {
                        if let ast::ModuleItem::NetVar(d) = it {
                            sc.collect_var_init_drivers(d);
                        }
                    }
                    sc.flush_block_local_inits();
                    sc.pending_var_inits = saved_pending;
                });
                for it in &decl.body {
                    match it {
                        ast::ModuleItem::ContAssign(ca) => self.elaborate_cont_assign(ca),
                        ast::ModuleItem::Proc(pb) => {
                            if self.try_elab_task(pb) {
                                continue;
                            }
                            let proc = self.lower_user_proc(pb);
                            self.push_process(proc);
                        }
                        ast::ModuleItem::NetVar(d) => self.elaborate_net_init_drivers(d),
                        ast::ModuleItem::Modport(_) => {} // binding enforces dirs
                        // §3.b `iface-subr`: DEFINITIONS, not logic — collected above
                        // (`register_declared_routine`, and `const_func_table` for the
                        // constant lane), classified and framed by `lower_frame_funcs`,
                        // expanded at their call sites. The module lane's Logic loop
                        // says exactly this (`instance.rs`). No-op here.
                        ast::ModuleItem::Func(_) | ast::ModuleItem::Task(_) => {}
                        ast::ModuleItem::Error(_)
                        | ast::ModuleItem::Param(_)
                        | ast::ModuleItem::PortDecl(_)
                        | ast::ModuleItem::Genvar { .. } => {}
                        // Applied above, around the parameter bind.
                        ast::ModuleItem::Import(_) => {}
                        other => {
                            let what = match other {
                                ast::ModuleItem::Instance(_) => "nested instances",
                                ast::ModuleItem::Generate(_) => "generate blocks",
                                ast::ModuleItem::Typedef(_) => "typedefs",
                                ast::ModuleItem::Defparam(_) => "defparam",
                                _ => "this construct",
                            };
                            self.error(
                                MsgCode::ElabUnsupported,
                                &format!("{what} inside an interface are outside the MVP"),
                            );
                        }
                    }
                }
                self.iface_insts.insert(path.clone(), iface_name.clone());
                // Below the Logic loop, not between the two passes: the maps have to
                // answer the same way in both, which is the half of the prerequisite
                // that is about POSITION rather than about the maps themselves.
                self.decl_pos = saved_decl_pos;
                self.decl_pos_scope = saved_dpos_scope;
                self.decl_pos_range = saved_dpos_range;
                self.decl_block_locals = saved_dbl;
                self.scoped_block_locals = saved_scoped_blocks;
                self.scoped_gather = saved_scoped_gather;
                self.scoped_gather_fed = saved_scoped_fed;
                self.per_entry_block_locals = saved_per_entry_blocks;
                self.coalesced_block_locals = saved_coalesced;
                self.local_decl_names = saved_local_names;
                self.restore_params(saved_params);
                // §3.b: below the Logic loop for the same reason the maps are — the
                // routine tables have to answer the same way in the Nets pass, the
                // frame lowering and the Logic pass.
                self.inst_stack.pop();
                // §3.b, round-2 delta: hand this window's static scoped frames to the
                // next sibling instance of the same parent BEFORE the parent's own
                // tables come back — see [`StaticScopedCarry`].
                self.release_static_scoped_frames(&parent_inst);
                self.restore_routine_scope(saved_rtn);
                self.cur_prefix = saved_prefix;
            }
            // v6 ②: header-port connections wire LATE (all parent nets exist
            // by pass 8); the early 4c call leaves them for this pass.
            if wire_phase {
                let has_conns = match &item.conns {
                    // `.*` alone matches zero ports on a port-less module → not
                    // "connections given", so the wildcard does not count here.
                    ast::PortConnList::Named(v, _) => !v.is_empty(),
                    ast::PortConnList::Positional(v) => !v.is_empty(),
                };
                let has_ports = !matches!(&decl.ports, ast::PortList::None)
                    && !matches!(&decl.ports, ast::PortList::Ansi(v) if v.is_empty());
                if has_conns && !has_ports {
                    self.error(
                        MsgCode::ElabPortMismatch,
                        "connections on a portless interface instance",
                    );
                    continue;
                }
                if has_ports {
                    let binding = match &item.conns {
                        ast::PortConnList::Named(v, wc) => PortBinding::Named(v, *wc),
                        ast::PortConnList::Positional(v) => PortBinding::Positional(v),
                    };
                    let saved_prefix = std::mem::replace(&mut self.cur_prefix, path.clone());
                    let parent = saved_prefix.clone();
                    self.wire_ports(&decl, binding, &parent, false);
                    self.cur_prefix = saved_prefix;
                }
            }
        }
    }
}
