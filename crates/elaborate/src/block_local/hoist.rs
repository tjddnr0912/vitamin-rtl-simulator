//! block-local HOISTING — creating the flattened module nets for a procedural
//! block's declarations, and collecting their declaration initializers.
//!
//! Split from `block_local.rs` (R17). The acceptance gates these call live in
//! `gate.rs`.

use super::*;

impl Elaborator<'_> {
    /// §6.8/§6.21: collect a block-local declaration's non-constant initializers into
    /// the t0 var-init sweep. `scope` is the `$blk$<lo>` segment when the declaration
    /// was given its own scope, `None` for the flattened path.
    ///
    /// §4.5.251: the scoped path used to skip this entirely, which is why a scoped
    /// `byte m[] = '{…}` came back EMPTY and why the same-name widening had to exclude
    /// every initializer-bearing declaration. The push target is the only difference —
    /// a scoped init is recorded under its own prefix and replayed there.
    fn collect_block_local_decl_inits(
        &mut self,
        d: &ast::NetVarDecl,
        span: ast::Span,
        scope: Option<&str>,
    ) {
        // §6.8/§6.21: a NON-constant PROCESS block-local initializer
        // (`begin logic x = g+1; …`) is STATIC-lifetime — applied ONCE
        // at time 0 (the block-local net is module-flattened), NOT on
        // each block entry. So it rides the SAME synthesized var-init
        // `initial` as a module-scope non-const var-init (matches
        // iverilog for an `always`/`for` body, which freezes the t0
        // value). §4.5.257: a constant init rides the same sweep — `net.init` carries only
        // the type default now, so skipping one here would drop the value outright.
        // A scalar `string s = expr;` block-local has no foldable
        // net.init field, so it always rides this t0 pre-sweep (a
        // dimensioned string was loud-rejected in `elaborate_netvar_decl`).
        let scalar_string =
            matches!(d.kind, ast::NetVarKind::String) && d.range.is_none() && d.packed.is_empty();
        if netvar_kind_is_var(d.kind) || scalar_string {
            for name in &d.names {
                let Some(init) = &name.init else { continue };
                let push = if scalar_string {
                    // A scalar string (no dims) rides the t0 pre-sweep. A
                    // string DYNAMIC array (`string s[] = '{…}`) rides it too:
                    // its `'{…}` is expanded by the flush (`new[N]` + element
                    // writes) exactly like the module-scope path
                    // (`collect_var_init_drivers`'s `is_dyn_str_init`). Without
                    // this the dimensioned-string branch dropped the init here
                    // (`name.unpacked` is non-empty → `push` false), leaving the
                    // block-local array silently EMPTY while the identical
                    // module-scope decl worked (a pre-existing silent-wrong).
                    // Other string dims (fixed / multi / non-`'{…}`) were
                    // loud-rejected at the decl, so they never reach here.
                    name.unpacked.is_empty()
                        || crate::string_array_route::is_dyn_string_container_init(
                            &name.unpacked,
                            init,
                        )
                } else {
                    // Mirror `collect_var_init_drivers`: a non-constant
                    // initializer rides the t0 pre-sweep. This INCLUDES an
                    // unpacked-array pattern (`int a[4] = '{1,2,3,4}`),
                    // whose synthesized `a = '{…}` is routed through
                    // `array_assign_special` — previously a bare
                    // `name.unpacked.is_empty()` guard silently dropped it.
                    // §4.5.257: a constant rides the t0 sweep like everything else now —
                    // `net.init` carries only the type default, so skipping it here would
                    // drop the value outright.
                    true
                };
                // r18 (family D): a per-entry local's initializer is emitted at
                // BLOCK ENTRY (the Logic-phase Block arm), so it must NOT also
                // ride the t0 static pre-sweep — that would double-init (and a
                // loop-var-reading init reads X at t0). Skip the push here.
                let per_entry = self
                    .per_entry_block_locals
                    .get(&span.lo)
                    .is_some_and(|s| s.contains(&name.name.name));
                if push && !per_entry {
                    let path = ast::HierPath {
                        segments: vec![name.name.clone()],
                        span: name.name.span,
                    };
                    // §4.5.254: EVERY block-local init goes to the one deferred
                    // list, tagged with its declaration offset. It cannot ride
                    // `pending_var_inits`, which is filled by the module-scope
                    // sweep that has not run yet — a block-local pushed there
                    // preceded module-scope inits it may read. Holding back only
                    // the STRING ones (r19) fixed that for strings while
                    // reordering them against their own block's non-strings.
                    self.push_block_local_init(
                        scope,
                        name.name.span.lo,
                        ast::Lvalue::Ident(path),
                        init.clone(),
                    );
                } else if scalar_string
                    && !name.unpacked.is_empty()
                    && self.has_fixed_string_array_storage(&name.name.name)
                {
                    // r19: a block-local FIXED string array (`string s[2] =
                    // '{…}`) — `push` is false for it (the gate above admits
                    // only a scalar or a `string s[]`), so expand it to one
                    // `s[k] = <elem>` per declared index here, into the same
                    // deferred string list so it lands after the module-scope
                    // string inits it may read.
                    //
                    // Gated on the decl having created the element storage,
                    // exactly like the module-scope collector — that is what
                    // keeps the two scopes from drifting (the class of bug
                    // that silently emptied a block-local `string s[] = '{…}`
                    // once before). Deliberately NOT gated on `per_entry`:
                    // that set only ever holds scalars and dyn/queue locals,
                    // so excluding it here would be dead code that silently
                    // opts INTO dropping the init if that set ever widens.
                    //
                    // T1-4: the FULL `unpacked`, exactly as the module-scope
                    // collector passes it — the two scopes share ONE expansion
                    // and must hand it the same shape, or a nested pattern
                    // expands here and not there.
                    if let Some(pairs) =
                        self.string_array_init_pairs(&name.name, &name.unpacked, init)
                    {
                        // One declaration ⇒ ONE order key for every element write, so
                        // the stable sort keeps them in the order the expansion built.
                        let lo = name.name.span.lo;
                        for (lhs, rhs) in pairs {
                            self.push_block_local_init(scope, lo, lhs, rhs);
                        }
                    }
                }
            }
        }
    }

    /// One block-local declaration's Nets-phase hoist (extracted so the caller can
    /// wrap it in a `cur_span` anchor).
    #[allow(clippy::too_many_arguments)]
    fn hoist_one_block_local(
        &mut self,
        d: &ast::NetVarDecl,
        decls: &[ast::NetVarDecl],
        stmts: &[ast::Stmt],
        span: ast::Span,
        ports: &ast::PortList,
        body: &[ast::ModuleItem],
    ) {
        // DUP (round-5): a colliding `automatic` block-local that the
        // pure pre-scan marked (disjoint blocks, no module-net collision,
        // no nesting) gets its OWN `$blk$<span>` scope so two blocks'
        // same-named locals become DISTINCT nets instead of aliasing
        // (was E3009). It still must pass per-entry definite-assignment
        // (each scoped block independently) to be byte-identical to the
        // static flatten — same gate as the non-colliding automatic path
        // below. `Fork` spans are never marked (gather skips them), so a
        // fork local always falls through. Everything unmarked keeps the
        // pre-existing behavior.
        //
        // §4.5.249: a STATIC dynamic-storage local qualifies too. Two
        // same-named dynamic locals in disjoint blocks are two distinct
        // variables that cannot share one flattened handle, so the pair was
        // loud whenever either side was static — scoping it is a pure
        // loud → support move. The per-entry-lifetime gate inside is keyed on
        // `d.lifetime`, so a static decl skips it exactly as it does on the
        // unscoped path; only the NET it gets changes.
        // Must match `gather_auto_block_locals` exactly — a decl scoped here but not
        // gathered there (or the reverse) breaks the invariant that EVERY colliding
        // occurrence of a name is scoped.
        //
        // §4.5.251: a scalar `string` qualifies too, and an initializer no longer
        // disqualifies either — the scoped path records its initializers under its own
        // prefix now and replays them there.
        // Must be implied BY `gather_auto_block_locals` — a decl scoped here but not
        // gathered there breaks the invariant that every colliding occurrence of a name is
        // scoped. The `string_local` term is character-identical in both. The dyn-dim term
        // is deliberately not: gather tests it per NAME, this tests it decl-ANY, so this
        // side is the weaker test and gather ⇒ hoist holds. The extra permissiveness is
        // absorbed by `block_local_scope_seg`, which only ever scopes a name gather marked.
        // R16 §3.5: a comma declaration is N INDEPENDENT declarators, but the collision
        // decision below reads `d.names.first()` and then applies its verdict to every
        // name. In `automatic int n = 0, n_skip = 0;` only `n` collides, yet `n_skip`
        // was rejected too — with a message naming an existing net of that name, which
        // does not exist anywhere in the design — and then the whole declaration was
        // dropped, so every later use of `n_skip` reported "undeclared net/variable" one
        // line below its declaration. One `int n;` at module scope produced eight
        // diagnostics, seven of them about a variable with nothing wrong with it.
        //
        // Handled by splitting only when the declarators DISAGREE. A declaration whose
        // names all collide, or none of which do, takes the original path unchanged, so
        // this cannot perturb any design that elaborates today — the split is reachable
        // only from the mixed case, which is broken in every instance.
        if d.names.len() > 1 {
            let collides =
                |s: &Self, n: &ast::DeclName| s.symbols.contains_key(&s.fq(&n.name.name));
            let first = collides(self, &d.names[0]);
            if d.names.iter().any(|n| collides(self, n) != first) {
                for n in &d.names {
                    let one = ast::NetVarDecl {
                        names: vec![n.clone()],
                        ..d.clone()
                    };
                    self.hoist_one_block_local(&one, decls, stmts, span, ports, body);
                }
                return;
            }
        }
        let string_local =
            matches!(d.kind, ast::NetVarKind::String) && d.range.is_none() && d.packed.is_empty();
        let dyn_storage = string_local
            || d.names.iter().any(|n| {
                n.unpacked.iter().any(|dim| {
                    matches!(dim, ast::Dim::Dyn | ast::Dim::Queue(_) | ast::Dim::Assoc(_))
                })
            });
        // A block-local SHADOWING a module-scope name earns a scope too, for the
        // reason `gather_auto_block_locals` records: the net it would flatten onto is
        // the shadowed one. The three terms are the three kinds `gather` marks, and
        // `block_local_scope_seg` re-checks the marks, so this stays the weaker test.
        let shadows_module = d
            .names
            .iter()
            .any(|n| self.local_decl_names.contains(&n.name.name));
        if d.lifetime == Some(true) || dyn_storage || shadows_module {
            if let Some(seg) = self.block_local_scope_seg(span, d) {
                for n in &d.names {
                    let nm = &n.name.name;
                    let read_in_sibling_init = decls
                        .iter()
                        .flat_map(|dd| dd.names.iter())
                        .any(|nn| nn.init.as_ref().is_some_and(|e| !expr_no_ref(e, nm)));
                    // r18 (family D): a per-entry-safe local (its init re-runs at
                    // block entry) — supported even on a `$blk$`-scoped net.
                    let per_entry = self
                        .per_entry_block_locals
                        .get(&span.lo)
                        .is_some_and(|s| s.contains(nm));
                    // BL1 (round-19): a const-folding, never-reassigned local is
                    // byte-identical to the static flatten — skip the loud (see the
                    // non-scoped gate below for the full rationale). §4.5.257 changed the
                    // MECHANISM, not the conclusion: the constant no longer rides
                    // `net.init`, it is applied once by the pre-arm initialization phase
                    // before any process runs, and `stmt_never_assigns_ident` still
                    // guarantees nothing overwrites it afterwards.
                    let const_immune = n.init.as_ref().is_some_and(|init| {
                        let (w, ..) = self.range_to_dims(d.kind, d.range.as_ref(), d.signed);
                        fold_init(init, w).is_some() || self.const_eval_in_scope(init).is_some()
                    }) && stmt_never_assigns_ident(stmts, nm);
                    let elem_bounds = self.fixed_elem_bounds(n);
                    // The gate is about the `automatic` PER-ENTRY lifetime, so
                    // it applies only to an `automatic` decl — the same
                    // condition the unscoped path below wraps it in. A STATIC
                    // decl reaches this arm only since §4.5.249 (a static
                    // dynamic-storage local now earns a `$blk$` net too), and
                    // for it there is no per-entry reset to be unfaithful to.
                    if d.lifetime == Some(true) && !per_entry && !const_immune {
                        let sole = !self.coalesced_block_locals.contains(nm);
                        let bw = self.decl_bit_width(d, n);
                        let da =
                            self.block_local_definitely_assigned(stmts, nm, elem_bounds, sole, bw);
                        if n.init.is_some() || read_in_sibling_init || da.is_err() {
                            self.deny_per_entry_lifetime(nm, da.err());
                        }
                    }
                }
                // §4.5.255: the collector runs INSIDE the scope, so every prefix-sensitive
                // question it asks is answered in the prefix the declaration was created
                // in. That is what a routed `string s[2]` needs — its storage is registered
                // under `…$blk$<lo>.s`, and `has_fixed_string_array_storage` asked from the
                // module prefix said "no storage" and dropped the initializer. It also
                // aligns the collector's const-fold test with the one `elaborate_netvar_decl`
                // just made, which had been resolving names in a different scope.
                // `scoped_init_key(None)` inside the scope is the same key
                // `scoped_init_key(Some(seg))` produced outside it.
                self.with_scope(&seg, |s| {
                    s.elaborate_netvar_decl(d, ports, body, true);
                    s.mark_automatic_local_nets(d);
                    s.collect_block_local_decl_inits(d, span, None);
                });
                return;
            }
        }
        // v1 flattens block-locals into the module namespace (no
        // per-block scope). If a local name was already created by an
        // EARLIER block, skip re-creating it rather than erroring
        // "redeclared" — two SEQUENTIAL named blocks reusing the same
        // temp name (`integer local_v;`) then share one net, which is
        // correct since they never overlap in time.
        // r19: a module-scope FIXED string array (`string sa[2]`) occupies its
        // bare NAME while registering NO net under it — only the per-element
        // `<name>$sae$<i>` nets — so the `existing` probe below cannot see it
        // and a block-local `string` of that name silently flattened onto it.
        // Every module-scope `sa[i]` then resolved to the block-local scalar
        // instead: `sa[0]="zz"` became a `putc` byte-write and read back "".
        //
        // Gated on the STRING kind, deliberately — the same discipline as the
        // `new_str_read` gate below, and for the same reason. A NON-string
        // local of that name mostly coalesces harmlessly (it occupies `t.sa`
        // while the array's storage stays `t.sa$sae$i`), so rejecting on the
        // bare name collision alone turned a dozen byte-correct designs loud —
        // `logic [7:0] sa;`, `logic [7:0] sa[2];`, `int`/`real sa;`, multi-name
        // decls — every one of which iverilog runs and vita already got right.
        // The residual non-string shapes that ARE wrong (a scalar local whose
        // name the block index-selects; some named-block array cases) are
        // pre-existing and recorded in ROADMAP §2: separating them from the
        // correct ones needs a per-shape hazard model, not a name match.
        //
        // T1: keyed on `has_fixed_string_array_storage`, which covers BOTH
        // representations. Keying it on `string_array_elems` alone stopped
        // firing once zero-based arrays routed to the dyn form, and the
        // alias came straight back in a NEW shape — the module's own
        // `sa[0]="zz"` and its read-back resolved through DIFFERENT
        // resolvers (the write reached the block-local scalar, the read the
        // routed array), so `R=zz,yy` became a silent `R=,` at exit 0.
        let shadowed_string_array = if matches!(d.kind, ast::NetVarKind::String) {
            d.names
                .iter()
                .find(|n| self.has_fixed_string_array_storage(&n.name.name))
                .map(|n| n.name.name.clone())
        } else {
            None
        };
        if let Some(nm) = shadowed_string_array {
            // Name the scope the collision is actually IN. `has_fixed_string_array_storage`
            // resolves outward from `cur_prefix`, so the other declaration can live in a
            // generate scope or an enclosing instance, and saying "module scope" sent the
            // reader to the wrong place.
            let scope = if self.cur_prefix.is_empty() {
                "the enclosing scope".to_string()
            } else {
                format!("`{}`", self.cur_prefix)
            };
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "a block-local `{nm}` collides with a string ARRAY of the same name \
                     visible in {scope}; v1 flattens block-locals into the enclosing \
                     namespace and cannot give this one its own scope (the two would \
                     alias) — rename it",
                ),
            );
        }
        let existing = d
            .names
            .first()
            .and_then(|n| self.symbols.get(&self.fq(&n.name.name)).copied());
        if let Some(net) = existing {
            // ⓑ-breadth fix: a SCALAR local safely coalesces (the net
            // is just overwritten in time), but a DYNAMIC-STORAGE local
            // (queue/dyn-array/assoc/string) is backed by a persistent
            // heap that is NOT reset on block entry — sharing one net
            // across two blocks leaks the first block's elements into
            // the second (a silent-wrong the array reductions/`size()`
            // then compute over). v1 has no per-block scope to give them
            // distinct heaps, so reject loudly rather than miscompute.
            // Also fire when the EXISTING net is a plain scalar but THIS
            // (later) block's decl is a `string` that the block READS
            // (`begin logic s; end  begin string s; …=s[0]; end`): the
            // string coalesces onto the packed net and reading it is
            // silent-wrong. A WRITE-ONLY string coalesces harmlessly (its
            // truncated write is discarded), so gate on a READ to avoid
            // over-rejecting the common param-gated / dead-store shape.
            let new_str_read = matches!(d.kind, ast::NetVarKind::String)
                && d.names.first().is_some_and(|n| {
                    let nm = &n.name.name;
                    stmts.iter().any(|st| stmt_reads_ident(st, nm))
                                    // A SIBLING decl in THIS block can read the string
                                    // in its own initializer (`begin string s; int x =
                                    // s[0]; end`) — that read lives in a decl, not in
                                    // `stmts`, so scan the block's decl inits too.
                                    || decls.iter().flat_map(|dd| dd.names.iter()).any(|nn| {
                                        nn.init
                                            .as_ref()
                                            .is_some_and(|e| rvalue_reads_ident(e, nm))
                                    })
                });
            if self.is_dyn_handle_net(net) || self.is_string_net(net) || new_str_read {
                // Name the identifier and say what makes THIS one different
                // from the same-named locals that are fine: the two decls
                // must both be `automatic` (and neither nested inside the
                // other) to earn distinct `$blk$` nets. Without the name, N
                // of these in one run are indistinguishable — which is
                // exactly why the round-20 report could not narrow its 81.
                let nm = d
                    .names
                    .first()
                    .map(|n| n.name.name.as_str())
                    .unwrap_or("<unnamed>");
                // R16 §4-2: the old text ended "this one is `automatic`, so the OTHER is
                // not" — an INFERENCE from "the pair was not scoped", and a false one.
                // Scoping is withheld for several reasons, and in the reported case BOTH
                // declarations were spelled `automatic`; the reader was sent looking for
                // a static twin that did not exist. State what is known — this
                // declaration's own lifetime — and list the conditions without claiming
                // which one failed.
                let lifetime = match d.lifetime {
                    Some(true) => "this one is declared `automatic`",
                    Some(false) => "this one is declared `static`",
                    None => "this one takes the enclosing default lifetime",
                };
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "the dynamic-storage local `{nm}` (queue / dynamic array / \
                         associative array / string) is declared under the same name in \
                         another block, and this pair cannot be given distinct storage \
                         ({lifetime}). A pair earns distinct storage when both are \
                         `automatic`, or both are dynamic (or scalar-`string`), AND \
                         neither declaring block encloses the other. As written both \
                         would share one flattened handle and one heap; make both \
                         `automatic`, or rename one"
                    ),
                );
            }
            // GAP-D completeness (adversarial find): an `automatic`
            // block-local whose name COLLIDES with an existing net (a
            // module-scope net, or an earlier sibling block-local) is
            // ALIASED onto that net by the v1 flatten — this both defeats
            // the automatic's required distinct per-entry storage AND
            // BYPASSES the definite-assignment gate below (this `continue`
            // skips it, so a read-before-write colliding automatic would
            // be silently accepted with the shared/persisted value).
            // v1 has no per-block scope to give the shadowing automatic
            // its own storage, so reject LOUD rather than silently alias
            // (correct-or-loud) — the workaround is a distinct name.
            if d.lifetime == Some(true) {
                for n in &d.names {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "an `automatic` block-local `{}` collides with an \
                                         existing net of the same name; v1 flattens \
                                         block-locals into the module namespace and cannot \
                                         give the `automatic` its own per-entry storage (it \
                                         would alias the shadowed net) — rename it",
                            n.name.name
                        ),
                    );
                }
            } else {
                // A STATIC scalar block-local coalescing onto an EXISTING
                // same-named net (the v1 flatten "reuse the net in time" path)
                // matches iverilog's distinct-per-scope variable ONLY when
                // (a) its TYPE equals the shared net's AND (b) it is definitely
                // assigned before any read here. A type mismatch (different
                // width or signedness) makes THIS block read/write the shared
                // net with the WRONG type — wrong `%d` sign, wrong `>>>`
                // arithmetic, wrong `%h`/`$bits` width. A read-before-write
                // observes the PRIOR block's leftover value instead of the X a
                // fresh variable would hold. Both are silent-wrong; v1 has no
                // per-block scope to give this local distinct typed storage, so
                // loud (correct-or-loud). The SAFE same-type + definitely-
                // assigned coalesce (common `for`/`tmp` name reuse) is
                // unaffected. dyn/string collisions were already loud above;
                // `range_to_dims` is packed-only, so skip them here.
                let (nw, _, _, nsig) = self.range_to_dims(d.kind, d.range.as_ref(), d.signed);
                for n in &d.names {
                    let nm = &n.name.name;
                    // A name ALSO declared at MODULE scope is a legitimate
                    // SHADOW (`outer_s x; … begin inner_s x; … end`), handled
                    // by the struct/enum/typedef shadow-scoping — NOT a
                    // sibling block-local coalesce. Skip it: this guard targets
                    // block-vs-block (two disjoint blocks reusing one flattened
                    // net), where the colliding name is a pure block-local.
                    if self.local_decl_names.contains(nm) {
                        return;
                    }
                    let Some(ex) = self.symbols.get(&self.fq(nm)).copied() else {
                        return;
                    };
                    if self.is_dyn_handle_net(ex)
                        || self.is_string_net(ex)
                        || matches!(d.kind, ast::NetVarKind::String)
                    {
                        return;
                    }
                    let (ew, esig) = {
                        let e = &self.nets[ex as usize];
                        (e.width, e.signed)
                    };
                    let elem_bounds = self.fixed_elem_bounds(n);
                    let bw = self.decl_bit_width(d, n);
                    if nw != ew || nsig != esig {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "block-local `{nm}` is declared with a different \
                                             width/signedness than a same-named block-local in \
                                             another block; v1 flattens both to one module net \
                                             and cannot hold two types (the shared net would be \
                                             read/written with the wrong sign or width) — rename \
                                             one"
                            ),
                        );
                    } else if let Err(g) =
                        // `sole_writer: false` — the other block's write is exactly the
                        // leftover this gate is about, so "never written HERE" proves
                        // nothing.
                        self.block_local_definitely_assigned(
                            stmts,
                            nm,
                            elem_bounds,
                            false,
                            bw,
                        )
                    {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "block-local `{nm}` shares one flattened net with a \
                                             same-named block-local in another block but is READ \
                                             before it is assigned here — it would observe the \
                                             other block's leftover value, not a fresh variable's \
                                             default; assign it before use, or rename one"
                            ),
                        );
                        self.note_da_gave_up(nm, g);
                    }
                }
            }
            return;
        }
        // GAP-D soundness: a procedural block-local `automatic` is
        // byte-identical to the static flattening ONLY when its
        // per-entry reset/re-init is dead. An initializer (must re-run
        // each entry) or a read-before-write use (observes the reset
        // value) cannot be honored without a per-block frame, so reject
        // LOUD rather than silently give static semantics. The
        // static-equivalent case (`automatic t x; x = e; … x …`) is
        // accepted — it flattens correctly.
        if d.lifetime == Some(true) {
            for n in &d.names {
                let nm = &n.name.name;
                // A sibling block-local's initializer that reads this
                // automatic var (`automatic int v; int w = v;`) observes
                // its entry value too — not just the block statements.
                // Use the CONSERVATIVE `expr_no_ref` (not the shared
                // under-detecting `expr_reads_ident`, which `_ => false`s
                // on `ArrayMethodWith`/`Dist`/… and could miss a hidden
                // read) so this gate is sound like the statement scan.
                let read_in_sibling_init = decls
                    .iter()
                    .flat_map(|dd| dd.names.iter())
                    .any(|nn| nn.init.as_ref().is_some_and(|e| !expr_no_ref(e, nm)));
                // r18 (family D): a per-entry-safe automatic-with-init local
                // (not under a fork, no collision — see `compute_per_entry_
                // block_locals`) is now supported: its initializer re-runs at
                // BLOCK ENTRY (emitted by the Logic-phase Block arm), so skip
                // the loud reject here.
                let per_entry = self
                    .per_entry_block_locals
                    .get(&span.lo)
                    .is_some_and(|s| s.contains(nm));
                // BL1 (round-19): an `automatic` block-local whose initializer
                // FOLDS TO A CONSTANT and which is NEVER reassigned in the block is
                // byte-identical to the static flatten — the constant is applied once by
                // the pre-arm initialization phase and never rewritten, so it is
                // CONCURRENCY-IMMUNE even under a `fork` (module-process forks have no
                // frame arena, but every activation reads the SAME constant off one
                // shared net). Skip the loud for it; do NOT mark per-entry — a
                // constant needs no re-init, and the const `net.init` handles t0.
                let const_immune = n.init.as_ref().is_some_and(|init| {
                    let (w, ..) = self.range_to_dims(d.kind, d.range.as_ref(), d.signed);
                    fold_init(init, w).is_some() || self.const_eval_in_scope(init).is_some()
                }) && stmt_never_assigns_ident(stmts, nm);
                let elem_bounds = self.fixed_elem_bounds(n);
                if !per_entry && !const_immune {
                    let sole = !self.coalesced_block_locals.contains(nm);
                    let bw = self.decl_bit_width(d, n);
                    let da = self.block_local_definitely_assigned(stmts, nm, elem_bounds, sole, bw);
                    if n.init.is_some() || read_in_sibling_init || da.is_err() {
                        self.deny_per_entry_lifetime(nm, da.err());
                    }
                }
            }
        } else {
            // R18-X1: the FIRST declaring block of a coalesced name, when the decl is
            // not `automatic` (a plain `int v;` never reaches the gate above at all).
            //
            // The coalesce guard earlier in this function only sees the SECOND and
            // later blocks — it keys on the net already existing — so this block, which
            // shares the very same net, was analysed as its sole writer and accepted
            // whatever it did. That is where `A v=99` came from where iverilog prints
            // `A v=1`: write, call a task that suspends, read back, and the sibling
            // block's write to the one net lands in between. `coalesced_block_locals`
            // is order-independent, so both blocks now get the same question.
            for n in &d.names {
                let nm = &n.name.name;
                if !self.coalesced_block_locals.contains(nm) {
                    continue;
                }
                let elem_bounds = self.fixed_elem_bounds(n);
                let bw = self.decl_bit_width(d, n);
                if let Err(g) =
                    self.block_local_definitely_assigned(stmts, nm, elem_bounds, false, bw)
                {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "block-local `{nm}` shares one flattened net with a \
                             same-named block-local in another block but is READ before \
                             it is assigned here — it would observe the other block's \
                             leftover value, not a fresh variable's default; assign it \
                             before use, or rename one"
                        ),
                    );
                    self.note_da_gave_up(nm, g);
                }
            }
        }
        self.elaborate_netvar_decl(d, ports, body, true);
        self.mark_automatic_local_nets(d);
        // Record the FLATTENED key. This net belongs to ONE process, but
        // the v1 flatten publishes it under the enclosing prefix's bare
        // name — inside a generate block that is `t.g.W`, a DIFFERENT key
        // from the module constant `t.W`. Name resolution's inner-net-wins
        // rule must not treat it as a legitimate shadow of an outer
        // constant, or every OTHER reader in that generate scope (a
        // sibling `initial`, a continuous assign, an inner generate) picks
        // up one process's private variable instead of the constant.
        for n in &d.names {
            let k = self.fq(&n.name.name);
            // `span` is the DECLARING BLOCK's, not the declaration's: a reader inside
            // that range sees this local, one outside sees whatever the enclosing scope
            // binds. Recorded here because this is the only place that knows both.
            self.hoisted_block_local
                .entry(k)
                .or_default()
                .push((span.lo, span.hi));
        }
        // Review S2: if THIS BLOCK has a `$blk$` scope, every one of its declaration
        // initializers goes into that group — not just the scoped names. Splitting them
        // put the scoped half after the whole main sweep, so `int a = $random; int q[$]
        // = '{$random};` in one block handed `a` the first draw and `q` the fourth.
        // Routing the block as a unit keeps declaration order inside it, and the group
        // still runs after the module-scope inits (which is the ordering a block-local
        // needs anyway — it may read them, never the reverse).
        //
        // A non-scoped name lowered under the `$blk$` prefix still resolves: lookup walks
        // OUTWARD, so it finds its module-flattened net exactly as before.
        let blk_scope = self
            .scoped_block_locals
            .get(&span.lo)
            .map(|_| format!("$blk${}", span.lo));
        self.collect_block_local_decl_inits(d, span, blk_scope.as_deref());
    }

    /// Recursively create nets for every `begin…end`/`fork…join` block-local
    /// declaration reachable from a procedural-block body. v1 flattens these to
    /// module-scope nets (no per-process frame). Called in the Nets phase.
    pub(crate) fn hoist_block_local_nets(
        &mut self,
        s: &ast::Stmt,
        ports: &ast::PortList,
        body: &[ast::ModuleItem],
    ) {
        match s {
            ast::Stmt::Block {
                decls, stmts, span, ..
            }
            | ast::Stmt::Fork {
                decls, stmts, span, ..
            } => {
                self.deny_static_init_reading_per_entry(decls, *span);
                for d in decls {
                    // §4.5.249: the Nets-phase hoist never passes through `lower_stmt`,
                    // so anchor each declaration's diagnostics here. This is exactly the
                    // family the report could not narrow — 81 identical messages with no
                    // position and, for the same-name class, no identifier either.
                    let saved_span = self.cur_span.replace(d.span);
                    self.hoist_one_block_local(d, decls, stmts, *span, ports, body);
                    self.cur_span = saved_span;
                }
                // This block's per-entry locals are in scope for every NESTED block's
                // static initializers too (`begin automatic int c = 5; begin int z =
                // f(c); … end end`), so publish them for the recursion and take them
                // back on the way out.
                let pushed: Vec<String> = self
                    .per_entry_block_locals
                    .get(&span.lo)
                    .map(|s| {
                        s.iter()
                            .filter(|n| self.per_entry_in_scope.insert((*n).clone()))
                            .cloned()
                            .collect()
                    })
                    .unwrap_or_default();
                // R16 §3.4: recurse under THIS block's `$blk$` segment when it has one,
                // mirroring the Logic phase (`stmt_main.rs`), which lowers a scoped
                // block's body inside `with_scope("$blk$<lo>")` and therefore NESTS the
                // segments when an outer and an inner block are both scoped.
                //
                // The hoist used to recurse flat, so a scoped inner block's nets were
                // created at `t.$blk$<inner>.n` while its body was lowered under
                // `t.$blk$<outer>.$blk$<inner>` — the outward walk then missed them and
                // fell through to the module. That mismatch is what forced the classifier
                // to drop every candidate block nested inside another candidate, which is
                // why a name reused at ONE level worked and the same name reused at TWO
                // levels did not: the standard table-driven walker shape
                // (`foreach (files[fi]) begin automatic int fd = $fopen(…); … begin
                // <inner locals> end end`) repeated in sibling blocks.
                let seg = self
                    .scoped_block_locals
                    .contains_key(&span.lo)
                    .then(|| format!("$blk${}", span.lo));
                match seg {
                    Some(seg) => self.with_scope(&seg, |s| {
                        for st in stmts {
                            s.hoist_block_local_nets(st, ports, body);
                        }
                    }),
                    None => {
                        for st in stmts {
                            self.hoist_block_local_nets(st, ports, body);
                        }
                    }
                }
                for n in pushed {
                    self.per_entry_in_scope.remove(&n);
                }
            }
            ast::Stmt::If { then_s, else_s, .. } => {
                self.hoist_block_local_nets(then_s, ports, body);
                if let Some(e) = else_s {
                    self.hoist_block_local_nets(e, ports, body);
                }
            }
            ast::Stmt::Case { items, .. } => {
                for it in items {
                    let inner = match it {
                        ast::CaseItem::Match { body: b, .. } => b,
                        ast::CaseItem::Default { body: b, .. } => b,
                    };
                    self.hoist_block_local_nets(inner, ports, body);
                }
            }
            ast::Stmt::For { body: b, .. }
            | ast::Stmt::While { body: b, .. }
            | ast::Stmt::Repeat { body: b, .. }
            | ast::Stmt::Forever { body: b, .. } => {
                self.hoist_block_local_nets(b, ports, body);
            }
            _ => {}
        }
    }

    /// R17: record the nets just created for an `automatic` block-local declaration so
    /// a hierarchical reference to one can be rejected (IEEE 1800 §23.9 — an automatic
    /// variable has no static address to name; v1's flatten accidentally gives it one).
    /// Called right after the nets exist, in whatever scope the hoist created them.
    fn mark_automatic_local_nets(&mut self, d: &ast::NetVarDecl) {
        if d.lifetime != Some(true) {
            return;
        }
        for n in &d.names {
            if let Some(&id) = self.symbols.get(&self.fq(&n.name.name)) {
                self.automatic_local_nets.insert(id);
            }
        }
    }
}
