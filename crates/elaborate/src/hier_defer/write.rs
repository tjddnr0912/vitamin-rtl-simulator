//! deferred hierarchical WRITE resolution — whole-net and element/part-select write
//! targets, plus the guards a hierarchical reference has to pass (clocking inputs,
//! array parameters, `wire` targets, `automatic` block-locals).
//!
//! Split from `hier_defer.rs` (R17).

use super::*;

impl Elaborator<'_> {
    /// Every lvalue net a CONTINUOUS assign writes. Both write resolvers use it to
    /// tell which lane a sentinel came from: `collect_lval_chunks` is shared by the
    /// procedural and the continuous lowering, so the sentinel itself carries no
    /// lane — but one deferral site produces exactly one chunk in exactly one
    /// arena, so the container it lands in IS the evidence. Needed because the two
    /// arenas are patched separately AND because the `wire` guard below is a rule
    /// about PROCEDURAL writes only.
    pub(crate) fn cont_assign_lval_nets(&self) -> std::collections::BTreeSet<u32> {
        self.cont_assigns
            .iter()
            .flat_map(|ca| ca.lhs.chunks.iter())
            .map(|c| c.net)
            .collect()
    }

    /// Record a deferred hierarchical WRITE target and return its sentinel net id
    /// (`HIER_WRITE_SENTINEL_BASE + index`). The chunk carries this sentinel until
    /// `resolve_deferred_hier_write` patches it. Falls back to a loud `resolve_net`
    /// (→ POISON) only if the sentinel range is exhausted (≈16M deferred writes —
    /// unreachable in practice).
    pub(crate) fn defer_hier_write(&mut self, path: &ast::HierPath) -> u32 {
        let idx = self.deferred_hier_write.len() as u32;
        if idx >= POISON_NET - HIER_WRITE_SENTINEL_BASE {
            return self.resolve_net(path);
        }
        self.deferred_hier_write.push(DeferredHierWrite {
            span: self.cur_span,
            prefix: self.cur_prefix.clone(),
            path: path.segments.iter().map(|s| s.name.clone()).collect(),
        });
        HIER_WRITE_SENTINEL_BASE + idx
    }

    /// Resolve the deferred hierarchical WRITE targets (`tb.dut.x = …`) once every
    /// instance's nets are in `symbols`. Mirrors `resolve_deferred_hier` (the read
    /// side) but patches `LvalChunk.net` across the statement arena: build a
    /// `sentinel → NetId` map (applying the write-context guards on the resolved
    /// net), then scan every lvalue-bearing statement and replace any sentinel
    /// chunk net. Whole-net only — a hierarchical element/part-select write never
    /// reaches here (it stays loud in the generic lvalue path).
    pub(crate) fn resolve_deferred_hier_write(&mut self) {
        let pending = std::mem::take(&mut self.deferred_hier_write);
        if pending.is_empty() {
            return;
        }
        let mut patch: std::collections::BTreeMap<u32, u32> = std::collections::BTreeMap::new();
        let ca_nets = self.cont_assign_lval_nets();
        let ambient = self.cur_span;
        for (i, d) in pending.iter().enumerate() {
            self.cur_span = d.span.or(ambient);
            let sentinel = HIER_WRITE_SENTINEL_BASE + i as u32;
            let real = match self.hier_lookup(&d.prefix, &d.path) {
                None => {
                    self.error_hier_unresolved(
                        &d.prefix,
                        &d.path,
                        format!(
                            "undeclared hierarchical write target `{}` (no such cross-instance net)",
                            d.path.join(".")
                        ),
                    );
                    POISON_NET
                }
                Some(net)
                    if self.hier_ref_to_automatic_local(net, &d.path.join("."), "write to") =>
                {
                    POISON_NET
                }
                Some(net) => {
                    // N4 §14.3: a clocking INPUT is read-only — a HIERARCHICAL drive
                    // from a parent (`dut.cb.sig = v`) must be loud, NOT a silent write
                    // to the holding reg (the in-module guard at `collect_lval_chunks`
                    // cannot see a cross-instance name, so it is caught HERE on the
                    // resolved net).
                    if self.clocking_hold_nets.contains(&net) {
                        // Through the read-only funnel, which names the signal the source
                        // wrote — the message used to hardcode `cb.sig` whatever the
                        // design called it.
                        self.deny_readonly_write(net, "drive");
                        POISON_NET
                    } else if self.const_param_nets.contains_key(&net) {
                        // A2a: a hierarchical write resolves AFTER lower_lvalue
                        // (sentinel chunk), so the funnel deny never saw the real
                        // net — enforce it here (`u1.RHO = …` incl. the [0:0]
                        // single-element shape that passes the static-array guard).
                        self.deny_readonly_write(net, "assign to");
                        POISON_NET
                    } else if self.event_nets.contains(&net)
                        || self.is_dyn_handle_net(net)
                        || self.net_is_static_array(net)
                    {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "hierarchical write of `{}` is unsupported (a named event, a \
                                 dynamic handle, or a whole unpacked array is not a plain \
                                 whole-net write target; a hierarchical element select is a \
                                 deferred follow-on)",
                                d.path.join(".")
                            ),
                        );
                        POISON_NET
                    } else if matches!(
                        self.nets.get(net as usize).map(|nv| &nv.kind),
                        Some(ir::NetKind::Wire)
                    ) && !ca_nets.contains(&sentinel)
                    {
                        // P1-9 (E3018) for the deferred path: a PROCEDURAL hierarchical
                        // write may not target a `wire` (iverilog rejects it too). A
                        // continuous assign is the opposite case — driving a wire is what
                        // `assign` is for, and both oracles run `assign u1.w = v;` — so
                        // the guard is keyed on the lane, not on the destination alone.
                        self.error(
                            MsgCode::ElabLvalueKind,
                            &format!(
                                "procedural hierarchical write to net `{}` (declare it reg/logic)",
                                d.path.join(".")
                            ),
                        );
                        POISON_NET
                    } else {
                        net
                    }
                }
            };
            patch.insert(sentinel, real);
            // Publish what this lane decided, so the end-phase decision can measure a
            // target holding this deferred piece (`edge_gate.rs` `chunk_width`). This
            // lane is WHOLE-NET only (see the doc above), so the resolved chunk is the
            // plain net.
            self.hier_resolved_chunk.insert(
                sentinel,
                ir::LvalChunk {
                    net: real,
                    word: None,
                    offset: None,
                    width: None,
                    kind: ir::SelKind::Bit,
                },
            );
        }
        self.cur_span = ambient;
        for s in &mut self.stmts {
            let chunks = match s {
                ir::Stmt::BlockingAssign { lhs, .. }
                | ir::Stmt::NonblockingAssign { lhs, .. }
                | ir::Stmt::Force { lhs, .. }
                | ir::Stmt::Release { lhs, .. } => &mut lhs.chunks,
                _ => continue,
            };
            for c in chunks {
                if let Some(&real) = patch.get(&c.net) {
                    c.net = real;
                }
            }
        }
        // …and the CONTINUOUS assigns, which are a separate arena and not `Stmt`s.
        // Scanning only `stmts` left `assign u1.x = v;` carrying its sentinel all the
        // way into the engine, where `chunk_width` indexes `nets[0xFF00_0000]` and
        // PANICS (exit 101) — below loud on the ladder, and it hit every direction
        // (self / down / up) with no generate block or array index involved.
        for ca in &mut self.cont_assigns {
            for c in &mut ca.lhs.chunks {
                if let Some(&real) = patch.get(&c.net) {
                    c.net = real;
                }
            }
        }
    }

    /// Record a deferred hierarchical element/bit-select WRITE; returns the sentinel
    /// net the placeholder `LvalChunk` carries until `resolve_deferred_hier_sel_write`
    /// rebuilds it. Falls back to `POISON_NET` (loud downstream) if the sentinel range
    /// is exhausted (~16M deferred element writes — unreachable in practice).
    pub(crate) fn defer_hier_sel_write(
        &mut self,
        path: Vec<String>,
        idx_eids: Vec<u32>,
        part: Option<HierPart>,
    ) -> u32 {
        let idx = self.deferred_hier_sel_write.len() as u32;
        if idx >= HIER_WRITE_SENTINEL_BASE - HIER_SEL_WRITE_SENTINEL_BASE {
            return POISON_NET;
        }
        self.deferred_hier_sel_write.push(DeferredHierSelWrite {
            span: self.cur_span,
            prefix: self.cur_prefix.clone(),
            path,
            idx_eids,
            part,
        });
        HIER_SEL_WRITE_SENTINEL_BASE + idx
    }

    /// Resolve the deferred hierarchical ELEMENT/bit-select WRITE targets
    /// (`dut.mem[i] = …`, `m.l.v[3] <= …`) once every instance's nets exist. The write
    /// twin of [`Self::resolve_deferred_hier_sel`]: resolve the base net, apply the
    /// write-context guards (no event / dynamic-handle source, no procedural write to a
    /// `wire`), REBUILD the full `LvalChunk` from the net's shape (array element word /
    /// packed bit-slice / vector bit), and replace every matching sentinel chunk in the
    /// statement arena. Runs BEFORE the multidriver scan so it sees real net ids.
    pub(crate) fn resolve_deferred_hier_sel_write(&mut self) {
        let pending = std::mem::take(&mut self.deferred_hier_sel_write);
        if pending.is_empty() {
            return;
        }
        let mut patch: std::collections::BTreeMap<u32, (ir::LvalChunk, String)> =
            std::collections::BTreeMap::new();
        let ca_nets = self.cont_assign_lval_nets();
        let ambient = self.cur_span;
        for (i, d) in pending.into_iter().enumerate() {
            self.cur_span = d.span.or(ambient);
            let sentinel = HIER_SEL_WRITE_SENTINEL_BASE + i as u32;
            let path = d.path.join(".");
            // T1-8: the routed string array lives under a MANGLED net name, so the same
            // side-map fallback the READ resolution uses is applied here — second, so an
            // ordinary net of that name still wins, and with the same commit-to-scope walk.
            let resolved = self
                .hier_lookup(&d.prefix, &d.path)
                .or_else(|| self.hier_resolve(&d.prefix, &d.path, &self.fixed_string_dyn_key));
            // R-SITE: the rebuilt chunk's index arithmetic is built under the mark of the
            // index it wraps (lowered with the statement; a continuous assign has none).
            let site = (d.idx_eids.first().copied())
                .or(d.part.map(|p| p.raw_off))
                .map_or_else(read_set::ReadMark::default, |e| self.mark_of_expr(e));
            let chunk = match resolved {
                None => {
                    self.error_hier_unresolved(
                        &d.prefix,
                        &d.path,
                        format!(
                            "undeclared hierarchical write target `{path}` (no such cross-instance net)"
                        ),
                    );
                    poison_chunk()
                }
                Some(net) if self.hier_ref_to_automatic_local(net, &path, "write to") => {
                    poison_chunk()
                }
                Some(net) => {
                    if self.clocking_hold_nets.contains(&net) {
                        // N4 §14.3: a clocking INPUT is read-only — a hierarchical
                        // SELECT drive (`dut.cb.sig[i] = v`) is loud too.
                        // Through the read-only funnel, which names the signal the source
                        // wrote — the message used to hardcode `cb.sig` whatever the
                        // design called it.
                        self.deny_readonly_write(net, "drive");
                        poison_chunk()
                    } else if self.const_param_nets.contains_key(&net) {
                        // A2a: the deferred element/bit/part-select write lane —
                        // the funnel deny saw only the sentinel, so enforce it on
                        // the resolved net (`u1.RHO[0] = …` was a silent mutation).
                        self.deny_readonly_write(net, "assign to");
                        poison_chunk()
                    } else if let Some(word) = (d.part.is_none() && !self.event_nets.contains(&net))
                        .then(|| {
                            self.with_site(site, |s| s.hier_dyn_container_word(net, &d.idx_eids))
                        })
                        .flatten()
                    {
                        // T1-8: a hierarchical element WRITE into a dynamic container
                        // (`u.s[0] = "x"`, `u.d[i] = v`, `u.q[i] = v`). The addressing is
                        // the READ's, verbatim — `hier_dyn_container_word` — and the chunk
                        // is the one the LOCAL dyn element write builds in `lvalue.rs`
                        // (word-indexed, no offset/width, neutral `Bit` tag). Nothing
                        // about the engine's element store cares how the name was reached,
                        // which is why the loud reject below was never about capability.
                        ir::LvalChunk {
                            net,
                            word: Some(word),
                            offset: None,
                            width: None,
                            kind: ir::SelKind::Bit,
                        }
                    } else if self.event_nets.contains(&net) || self.is_dyn_handle_net(net) {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "a hierarchical indexed write of `{path}` is unsupported (an event \
                                 or a dynamic-storage handle has no plain indexable write target)"
                            ),
                        );
                        poison_chunk()
                    } else if matches!(
                        self.nets.get(net as usize).map(|nv| &nv.kind),
                        Some(ir::NetKind::Wire)
                    ) && !ca_nets.contains(&sentinel)
                    {
                        // P1-9 (E3018): a PROCEDURAL hierarchical write may not target a
                        // `wire` — iverilog rejects it too (whole-net precedent). The
                        // continuous lane is exempt for the whole-net twin's reason:
                        // `assign u1.w[3:0] = v;` is a driver, not a procedural write.
                        self.error(
                            MsgCode::ElabLvalueKind,
                            &format!(
                                "procedural hierarchical write to net `{path}` (declare it reg/logic)"
                            ),
                        );
                        poison_chunk()
                    } else {
                        self.with_site(site, |s| {
                            s.build_hier_sel_write_chunk(net, &d.idx_eids, d.part, &path)
                        })
                    }
                }
            };
            // Same publish as the whole-net lane — here the REBUILT chunk is the
            // answer, which is why a hierarchical BIT-select needs no special case: its
            // rebuilt chunk is one bit wide, so the width the decision measures is 1.
            self.hier_resolved_chunk.insert(sentinel, chunk.clone());
            patch.insert(sentinel, (chunk, path));
        }
        self.cur_span = ambient;
        // A hierarchical element/bit-select is NOT a legal force/release target — the
        // local path rejects a bit/part-select force (`is_whole_single_net`), but the
        // placeholder sel-write chunk LOOKS whole at that check, so enforce parity here.
        let mut force_release_paths: Vec<String> = Vec::new();
        for s in &mut self.stmts {
            let force_release = matches!(s, ir::Stmt::Force { .. } | ir::Stmt::Release { .. });
            let chunks = match s {
                ir::Stmt::BlockingAssign { lhs, .. }
                | ir::Stmt::NonblockingAssign { lhs, .. }
                | ir::Stmt::Force { lhs, .. }
                | ir::Stmt::Release { lhs, .. } => &mut lhs.chunks,
                _ => continue,
            };
            for c in chunks {
                if let Some((rebuilt, path)) = patch.get(&c.net) {
                    if force_release {
                        force_release_paths.push(path.clone());
                        *c = poison_chunk();
                    } else {
                        *c = rebuilt.clone();
                    }
                }
            }
        }
        // …and the CONTINUOUS assigns (`assign u1.mem[i] = v;`), the second lvalue
        // arena. No force/release case here: neither is a continuous assign.
        for ca in &mut self.cont_assigns {
            for c in &mut ca.lhs.chunks {
                if let Some((rebuilt, _)) = patch.get(&c.net) {
                    *c = rebuilt.clone();
                }
            }
        }
        for path in force_release_paths {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "force/release target must be a whole net/variable (a hierarchical \
                     element/bit-select `{path}` is not a legal force/release target)"
                ),
            );
        }
    }

    // ── lvalue lowering ────────────────────────────────────────────
    /// SYS-READ dest guard: the just-lowered dest `eid` is a deferred
    /// hierarchical ELEMENT-select placeholder (`Signal{POISON_NET}` with a
    /// pending `deferred_hier_sel` record). The fixup rebuilds a READ select
    /// — never a write chunk — so the engine write silently VANISHES for
    /// these dests (measured: rc=1, value unchanged; iverilog writes it).
    /// Every SYS-READ special loud-rejects them; a WHOLE hierarchical dest
    /// (`deferred_hier`) keeps its working patched path.
    pub(crate) fn is_deferred_hier_sel_dest(&self, eid: u32) -> bool {
        self.deferred_hier_sel.iter().any(|d| d.eid == eid)
    }

    /// R17 (IEEE 1800 §23.9): reject a hierarchical reference that resolves to a net
    /// created by flattening an `automatic` block-local, and say so.
    ///
    /// The standard forbids this outright: an automatic variable lives in per-call
    /// storage with no static address, so there is nothing for a hierarchical name to
    /// denote. vita's v1 flatten publishes one under the module prefix anyway, which
    /// made `other_module.tb.a` resolve and silently read or write the block's
    /// storage. Measured against iverilog, which rejects the same program with
    /// "Hierarchical reference to automatically allocated item".
    ///
    /// Returns `true` when the reference was rejected (the caller poisons).
    pub(crate) fn hier_ref_to_automatic_local(&mut self, net: u32, path: &str, verb: &str) -> bool {
        if !self.automatic_local_nets.contains(&net) {
            return false;
        }
        self.error(
            MsgCode::ElabUnsupported,
            &format!(
                "cannot {verb} `{path}` hierarchically: it names an `automatic` \
                 block-local, whose storage is per block entry and has no hierarchical \
                 address (IEEE 1800 §23.9) — drop `automatic`, or move the declaration \
                 to module scope"
            ),
        );
        true
    }
}
