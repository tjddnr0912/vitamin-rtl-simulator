//! lvalue lowering — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;

// ── v3 port-wiring helpers ─────────────────────────────────────────
/// A whole-net lvalue chunk (no word/offset/width → drives the entire net).
pub(crate) fn whole_net_chunk(net: u32) -> ir::LvalChunk {
    ir::LvalChunk {
        net,
        word: None,
        offset: None,
        width: None,
        kind: ir::SelKind::Bit,
    }
}

/// A single-chunk whole-net lvalue.
pub(crate) fn whole_net_lvalue(net: u32) -> ir::Lvalue {
    ir::Lvalue {
        chunks: vec![whole_net_chunk(net)],
    }
}

/// True iff an lvalue is exactly ONE whole-net chunk (no bit/part-select, no
/// array word) — the only legal force/release target shape (IEEE §9.3.2).
pub(crate) fn is_whole_single_net(lv: &ir::Lvalue) -> bool {
    matches!(
        lv.chunks.as_slice(),
        [ir::LvalChunk {
            word: None,
            offset: None,
            ..
        }]
    )
}

impl Elaborator<'_> {
    /// P1-9 (E3018): assignment-kind legality. `is_proc=true` (a procedural `=`/
    /// `<=`) may not target a NET (`wire`); `is_proc=false` (a user `assign`) may
    /// not drive a VARIABLE (`reg`/`integer`/`real`). SV `logic` passes both ways
    /// (IEEE 1800 admits either one continuous driver or procedural writes).
    /// Called ONLY for user-written assignments — port-binding/decl-init synthetic
    /// cont-assigns are exempt (IEEE 1800 §23.3.3 var ports are legal).
    pub(crate) fn check_lvalue_kind(&mut self, lhs: &ir::Lvalue, is_proc: bool) {
        for c in &lhs.chunks {
            let Some(nv) = self.nets.get(c.net as usize) else {
                continue; // POISON_NET / post-error recovery chunk
            };
            let bad = if is_proc {
                matches!(nv.kind, ir::NetKind::Wire)
            } else {
                matches!(
                    nv.kind,
                    ir::NetKind::Reg
                        | ir::NetKind::Integer
                        | ir::NetKind::Real
                        | ir::NetKind::String
                )
            };
            if bad {
                let name = self
                    .symbols
                    .iter()
                    .find(|(_, &id)| id == c.net)
                    .map(|(n, _)| n.clone())
                    .unwrap_or_else(|| format!("#{}", c.net));
                let msg = if is_proc {
                    format!("procedural assignment to net `{name}` (declare it reg/logic)")
                } else {
                    format!("continuous assign drives variable `{name}` (declare it wire/logic)")
                };
                self.error(MsgCode::ElabLvalueKind, &msg);
            }
        }
    }

    /// Total bit width of a LOWERED lvalue (mirrors the engine's
    /// `lvalue_width`/backend `lvalue_width_of`): whole-net chunks read the net
    /// (element) width, bit selects are 1, part selects read their const width
    /// edge. Used to size intra-assignment capture temps EXACTLY.
    pub(crate) fn ir_lvalue_width(&self, lv: &ir::Lvalue) -> u32 {
        lv.chunks
            .iter()
            .map(|c| match c.kind {
                ir::SelKind::Bit => {
                    if c.offset.is_none() && c.width.is_none() {
                        // `.get()` (not index) so an error-recovery lvalue with an
                        // out-of-range net id degrades to 1 instead of panicking.
                        self.nets.get(c.net as usize).map(|n| n.width).unwrap_or(1)
                    } else {
                        1
                    }
                }
                _ => c
                    .width
                    .and_then(|eid| self.const_of_expr_u32(eid))
                    .unwrap_or(1),
            })
            .sum::<u32>()
            .max(1)
    }

    pub(crate) fn lower_lvalue(&mut self, lv: &ast::Lvalue) -> ir::Lvalue {
        let mut chunks = Vec::new();
        self.collect_lval_chunks(lv, &mut chunks);
        // A2a: a chunk rooting at a desugared array parameter — every lvalue
        // shape (whole/element/slice/concat part, procedural or continuous,
        // force/release, SYS-READ dest routed through here) funnels through
        // these chunks, so one check covers them all. First hit is enough.
        if !self.const_param_nets.is_empty() {
            for c in &chunks {
                if self.deny_const_param_write(c.net, "assign to") {
                    break;
                }
            }
        }
        // review F3: a string chunk inside a CONCAT lvalue has chunk width 0
        // — the runtime slice loop wrote an EMPTY piece (silently clearing
        // the string). Whole-string single-chunk stays the supported shape.
        if chunks.len() > 1 && chunks.iter().any(|c| self.is_string_net(c.net)) {
            self.error(
                MsgCode::ElabUnsupported,
                "a string inside a concatenation lvalue is outside the v7 scope",
            );
        }
        // A string ELEMENT chunk carries a non-None offset (a partial bit-write into
        // the materialized vector). The plain `s[i] = c` form is intercepted earlier
        // and routed to `.putc`; a string element reaching here came through a
        // single-element concat `{s[0]} = c` (which slips past the multi-chunk guard
        // above) — a silent-wrong packed bit-write. The supported whole-string write
        // has offset None, so this never fires on `s = …`. Honest-loud.
        if chunks
            .iter()
            .any(|c| self.is_string_net(c.net) && c.offset.is_some())
        {
            self.error(
                MsgCode::ElabUnsupported,
                "a string element write inside a concatenation lvalue is outside the \
                 v7 scope (a plain `s[i] = c` is supported)",
            );
        }
        ir::Lvalue { chunks }
    }

    pub(crate) fn collect_lval_chunks(&mut self, lv: &ast::Lvalue, out: &mut Vec<ir::LvalChunk>) {
        // v6 ②: every lvalue shape roots at one Ident — check the modport
        // write rule once per root (concat parts re-enter here per part).
        if let Some(path) = lval_root_path(lv) {
            let path = path.clone();
            self.check_modport_write(&path);
            // N4 §14.3: a clocking INPUT (`cb.sig`) is read-only — driving it is
            // illegal. Resolve quietly (the joined alias) and reject if it targets
            // a clocking-input holding net (never a silent write to the holding reg).
            if !self.clocking_hold_nets.is_empty() {
                let joined = path
                    .segments
                    .iter()
                    .map(|s| s.name.as_str())
                    .collect::<Vec<_>>()
                    .join(".");
                if let Some(id) = self.lookup_net_scoped(&joined) {
                    if self.clocking_hold_nets.contains(&id) {
                        self.error(
                            MsgCode::ElabUnsupported,
                            "cannot drive a clocking INPUT (`cb.sig` is read-only, §14.3)",
                        );
                    }
                }
            }
        }
        match lv {
            ast::Lvalue::Ident(path) => {
                // N7: a class field WRITE (`obj.field = v` / `this.field = v` /
                // bare member inside a method) → `LvalChunk{handle, word:field-id}`.
                // Checked BEFORE the hierarchical-write path. Whole-net writes of
                // the handle itself (ref-copy / `h = null`) are single-segment and
                // fall through to the normal net path below.
                if let Some((hnet, class, field)) = self.resolve_class_member(path) {
                    match self.class_field_id(&class, &field) {
                        Some((fid, f)) => {
                            // IEEE §8.18: loud-reject an out-of-scope WRITE of a
                            // local/protected field (never silently write inaccessible
                            // storage). Still emit the chunk so downstream width logic
                            // is unaffected — the error makes the run non-zero exit.
                            self.check_field_access(&class, &field, &f);
                            let word = self.const_u32_expr(fid, 32);
                            // Encode the FIELD width so `chunk_width` reports it (the
                            // chunk's net is the 32-bit handle; a `None` width would
                            // size the write context to 32 and TRUNCATE a >32-bit
                            // field). The engine's class-field write branch ignores
                            // offset/width and resizes to the field itself.
                            let off0 = self.const_u32_expr(0, 32);
                            let fw = self.const_u32_expr(f.width.max(1), 32);
                            out.push(ir::LvalChunk {
                                net: hnet,
                                word: Some(word),
                                offset: Some(off0),
                                width: Some(fw),
                                kind: ir::SelKind::PartIdxUp,
                            });
                        }
                        None => self.error(
                            MsgCode::ElabUnsupported,
                            &format!("class `{class}` has no member `{field}`"),
                        ),
                    }
                    return;
                }
                // An output/inout task formal written by an inlined body targets the
                // caller's net directly (symmetric with the read side in lower_expr).
                let net = if path.segments.len() == 1 {
                    self.out_subst_lookup(&path.segments[0].name)
                        .unwrap_or_else(|| self.resolve_net(path))
                } else {
                    // Multi-segment: a known dotted symbol (interface-member alias)
                    // resolves directly; an UNKNOWN hierarchical name is a cross-
                    // instance WRITE target (`tb.dut.x = …`) — defer it like the read
                    // side and patch the chunk once every instance's nets exist
                    // (`resolve_deferred_hier_write`, which applies the write guards).
                    let joined = path
                        .segments
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join(".");
                    // A2b-prereq F2: never write through a dotted hit on a
                    // package-var import alias (§26.3) — defer, and the
                    // hierarchical write resolver stays loud.
                    if let Some(id) = self
                        .lookup_net_scoped(&joined)
                        .filter(|_| !self.dotted_hit_is_pkg_alias(&joined))
                    {
                        id
                    } else {
                        out.push(ir::LvalChunk {
                            net: self.defer_hier_write(path),
                            word: None,
                            offset: None,
                            width: None,
                            kind: ir::SelKind::Bit,
                        });
                        return;
                    }
                };
                if self.event_nets.contains(&net) {
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a named event cannot be assigned (only `->e` triggers it)",
                    );
                }
                if self.is_dyn_handle_net(net) {
                    // v5 ⑥: whole-handle assignment (`d2 = d`, `assign q = …`)
                    // is outside the MVP (`d = new[n]` is intercepted earlier
                    // and never reaches the lvalue path).
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a dynamic-storage handle cannot be assigned as a whole (use `new[]`, methods or element writes)",
                    );
                }
                if self.net_is_static_array(net) {
                    // Phase-1.x ②: a whole unpacked array is only a write
                    // target in procedural ARRAY ASSIGNMENT (intercepted
                    // before lowering reaches here). Anywhere else — force,
                    // continuous assign, … — it used to write word 0 SILENTLY.
                    self.error(
                        MsgCode::ElabUnsupported,
                        "a whole unpacked array cannot be the write target in \
                         this context (v1: procedural array assignment only)",
                    );
                }
                out.push(ir::LvalChunk {
                    net,
                    word: None,
                    offset: None,
                    width: None,
                    kind: ir::SelKind::Bit, // neutral tag; offset/width None ⇒ whole net
                });
            }
            ast::Lvalue::BitSelect { base, index, .. } => {
                // N6: a fixed string-ARRAY element write `files[K] = ...` (CONST K) → a
                // WHOLE-net write of the K-th string element net (a string array is a set
                // of scalar string nets). A runtime index / OOB is loud (correct-or-loud).
                if let ast::Lvalue::Ident(p) = &**base {
                    if p.segments.len() == 1 {
                        if let Some(key) = self.walk_scopes_key_shadowed(&p.segments[0].name, |k| {
                            self.string_array_elems.contains_key(k)
                        }) {
                            let (min, max, elems) =
                                self.string_array_elems.get(&key).unwrap().clone();
                            match self.const_eval_in_scope(index) {
                                Some(k) if k >= min && k <= max => {
                                    out.push(whole_net_chunk(elems[(k - min) as usize]));
                                }
                                Some(k) => self.error(
                                    MsgCode::ElabUnsupported,
                                    &format!(
                                        "string-array index {k} is out of the declared \
                                         range [{min}:{max}]"
                                    ),
                                ),
                                None => self.error(
                                    MsgCode::ElabUnsupported,
                                    "a string-array element write requires a constant \
                                     index (a runtime index is a deeper follow-on)",
                                ),
                            }
                            return;
                        }
                    }
                }
                // v5 ⑥: dyn-handle element write (`d[i] = v`, `q[$] = v`,
                // `a[k] = v`) — handles never take the static chunk paths.
                if let ast::Lvalue::Ident(p) = &**base {
                    if p.segments.len() == 1 {
                        if let Some((net, kind)) = self.dyn_handle(&p.segments[0].name) {
                            let word = self.lower_dyn_index(net, kind, index);
                            out.push(ir::LvalChunk {
                                net,
                                word: Some(word),
                                offset: None,
                                width: None,
                                kind: ir::SelKind::Bit,
                            });
                            return;
                        }
                    }
                }
                // SYMMETRY with the RHS read: a `base[i]…[k]` chain rooted at an
                // ARRAY net writes the flat element word (first D indices, row-major)
                // with an optional trailing single bit-select. `LvalChunk.word` is an
                // ExprId, evaluated at write time, so `mem[k] = …` (runtime k) and
                // `g[i][j] = …` (2-D element) both work; `mem[k]`/`g[i][j]` are the
                // D==1/D==2 ends of one path. A scalar base falls through to plain
                // bit-select.
                if let Some((net, idxs)) = self.lval_array_chain(base, index) {
                    self.collect_array_write(net, &idxs, out);
                } else if let Some((net, idxs)) = self.lval_packed_chain(base, index) {
                    self.collect_packed_write(net, &idxs, out);
                } else if let Some((path, idx_asts)) = self.lval_hier_sel_chain(base, index) {
                    // HIER-REST①: a hierarchical element/bit-select write — the target
                    // net does not exist yet. Lower the indices NOW (full param/genvar/
                    // formal context) and defer the chunk rebuild to
                    // `resolve_deferred_hier_sel_write` (the write twin of the read-side
                    // `resolve_deferred_hier_sel`).
                    let idx_eids: Vec<u32> = idx_asts.iter().map(|e| self.lower_expr(e)).collect();
                    let sentinel = self.defer_hier_sel_write(path, idx_eids, None);
                    out.push(ir::LvalChunk {
                        net: sentinel,
                        word: None,
                        offset: None,
                        width: None,
                        kind: ir::SelKind::Bit,
                    });
                } else {
                    let net = self.lval_base_net(base);
                    let raw_off = self.lower_expr(index);
                    let offset = self.norm_offset_for_net(net, raw_off);
                    let width = self.const_u32_expr(1, 32);
                    out.push(ir::LvalChunk {
                        net,
                        word: None,
                        offset: Some(offset),
                        width: Some(width),
                        kind: ir::SelKind::Bit,
                    });
                }
            }
            ast::Lvalue::PartSelect { base, msb, lsb, .. } => {
                // N3.4: a part-select on a bare multi-dim PACKED net writes whole
                // outer-dim elements, not flat bits.
                if let Some(chunk) = self.try_packed_part_select_lval(base, msb, lsb) {
                    out.push(chunk);
                    return;
                }
                // HIER-REST-PS: a hierarchical part-select write (`dut.v[3:0]=…`,
                // array-element `dut.mem[i][3:0]=…`) — the target net does not exist
                // yet; defer with the (lowered) lsb offset + const width and rebuild.
                if let Some((path, idx_asts)) = self.lval_hier_chain(base) {
                    let idx_eids: Vec<u32> = idx_asts.iter().map(|e| self.lower_expr(e)).collect();
                    let lsb_id = self.lower_expr(lsb);
                    let msb_id = self.lower_expr(msb);
                    let width = self.width_from_msb_lsb_checked(msb, lsb, msb_id, lsb_id);
                    let part = HierPart {
                        raw_off: lsb_id,
                        width,
                        kind: ir::SelKind::PartConst,
                    };
                    let sentinel = self.defer_hier_sel_write(path, idx_eids, Some(part));
                    out.push(ir::LvalChunk {
                        net: sentinel,
                        word: None,
                        offset: None,
                        width: None,
                        kind: ir::SelKind::Bit,
                    });
                    return;
                }
                // `x[j][m:l]` / `arr[i][j][m:l]` — a part-select whose base is a
                // NESTED select landing on the LEAF packed dim (write twin of the
                // read's element-select + packed-flatten). The generic path below
                // flattens only unpacked dims and would reject this (E3009).
                if let Some(chunk) = self.try_nested_packed_part_lval(base, msb, lsb) {
                    out.push(chunk);
                    return;
                }
                // `g[i][j][msb:lsb] = …` — part-select WITHIN an array element word.
                // `lval_part_base` resolves the element (net + flat word); a scalar
                // base gives `(net, None)` ⇒ the classic `r[msb:lsb]` chunk.
                let (net, word) = self.lval_part_base(base);
                let lsb_id = self.lower_expr(lsb);
                let msb_id = self.lower_expr(msb);
                let asc = self.net_ascending(net);
                let width = self.width_from_msb_lsb_dir(msb, lsb, msb_id, lsb_id, asc);
                let offset = self.norm_offset_for_net(net, lsb_id);
                out.push(ir::LvalChunk {
                    net,
                    word,
                    offset: Some(offset),
                    width: Some(width),
                    kind: ir::SelKind::PartConst,
                });
            }
            ast::Lvalue::IndexedPart {
                base,
                offset,
                width,
                dir,
                ..
            } => {
                // N3.4 follow-on: constant indexed part-select on multi-dim packed
                // writes whole outer elements; variable offset is loud.
                if let Some(chunk) = self.try_packed_indexed_part_lval(base, offset, width, dir) {
                    out.push(chunk);
                    return;
                }
                let kind = match dir {
                    ast::PartDir::PlusColon => ir::SelKind::PartIdxUp,
                    ast::PartDir::MinusColon => ir::SelKind::PartIdxDown,
                };
                // HIER-REST-PS: a hierarchical indexed part-select write
                // (`dut.v[o+:w]=…`) — defer with the (lowered) raw offset + width.
                if let Some((path, idx_asts)) = self.lval_hier_chain(base) {
                    let idx_eids: Vec<u32> = idx_asts.iter().map(|e| self.lower_expr(e)).collect();
                    let raw_off = self.lower_expr(offset);
                    let w = self.lower_expr(width);
                    let part = HierPart {
                        raw_off,
                        width: w,
                        kind,
                    };
                    let sentinel = self.defer_hier_sel_write(path, idx_eids, Some(part));
                    out.push(ir::LvalChunk {
                        net: sentinel,
                        word: None,
                        offset: None,
                        width: None,
                        kind: ir::SelKind::Bit,
                    });
                    return;
                }
                let (net, word) = self.lval_part_base(base);
                let raw_off = self.lower_expr(offset);
                let off = self.norm_offset_for_net(net, raw_off);
                let w = self.lower_expr(width);
                // Ascending net: flip the indexed direction (the offset is already
                // normalized by `norm_offset_for_net`). Descending keeps `kind`.
                let kind = indexed_sel_kind(dir, self.net_ascending(net));
                out.push(ir::LvalChunk {
                    net,
                    word,
                    offset: Some(off),
                    width: Some(w),
                    kind,
                });
            }
            ast::Lvalue::Concat { parts, .. } => {
                // Flatten left→right (MSB-first source order) into the chunk list.
                for p in parts {
                    self.collect_lval_chunks(p, out);
                }
            }
            ast::Lvalue::Error(_) => {
                self.error(MsgCode::ElabUnsupported, "cannot lower parse-error lvalue");
                out.push(ir::LvalChunk {
                    net: POISON_NET,
                    word: None,
                    offset: None,
                    width: None,
                    kind: ir::SelKind::Bit,
                });
            }
        }
    }

    /// An lvalue select's base must reduce to a net Ident in v1 (no nested
    /// selects). Returns NetId; emits + returns [`POISON_NET`] otherwise.
    pub(crate) fn lval_base_net(&mut self, base: &ast::Lvalue) -> u32 {
        match base {
            ast::Lvalue::Ident(p) => self.resolve_net(p),
            _ => {
                self.error(
                    MsgCode::ElabUnsupported,
                    "nested lvalue select (v1: single-level)",
                );
                POISON_NET
            }
        }
    }

    /// Resolve the base of a part-select LHS to `(net, word)`. A bare `Ident` is a
    /// scalar (or 1-D word-0) base ⇒ `word = None`. A `g[i]…[k]` chain rooted at an
    /// array net is an ELEMENT part-select (`g[i][j][msb:lsb] = …`): all D indices
    /// flatten to the element word, and the part-select applies within it. Indexing
    /// fewer than D dims (partial slice) or more than D (bit-then-part) is loud.
    pub(crate) fn lval_part_base(&mut self, base: &ast::Lvalue) -> (u32, Option<u32>) {
        if let ast::Lvalue::BitSelect {
            base: b, index: i, ..
        } = base
        {
            // N3: a dyn-ARRAY element part-select base (`arr[i].field = …`) — the
            // element word is the runtime dyn index; the part-select offset/width then
            // apply WITHIN the element (the engine does a read-modify-write). A dyn
            // handle never takes the static `lval_array_chain` path (mirrors the
            // BitSelect whole-element write).
            if let ast::Lvalue::Ident(p) = b.as_ref() {
                if p.segments.len() == 1 {
                    if let Some((net, kind)) = self.dyn_handle(&p.segments[0].name) {
                        let word = self.lower_dyn_index(net, kind, i);
                        return (net, Some(word));
                    }
                }
            }
            if let Some((net, idxs)) = self.lval_array_chain(b, i) {
                let dims = self.net_dim_extents(net);
                let d = dims.len();
                if idxs.len() == d {
                    let word = self.flatten_word(&dims, &idxs, &[]);
                    return (net, Some(word));
                }
                self.error(
                    MsgCode::ElabUnsupported,
                    if idxs.len() < d {
                        "partial unpacked-array slice (v1: index every dimension)"
                    } else {
                        "bit-select then part-select on a multi-dim array lvalue (v1 unsupported)"
                    },
                );
                return (POISON_NET, None);
            }
        }
        (self.lval_base_net(base), None)
    }

    /// A loud POISON lvalue chunk — the write-side counterpart of
    /// [`Self::placeholder_expr`], used after a loud E3009 so the generic path does
    /// not silently write past / into the wrong net.
    pub(crate) fn poison_chunk() -> ir::LvalChunk {
        ir::LvalChunk {
            net: POISON_NET,
            word: None,
            offset: None,
            width: None,
            kind: ir::SelKind::Bit,
        }
    }

    /// The dynamic INDEX sub-expressions of an assignment LVALUE are reads that
    /// belong in an implicit `@(*)`/`always_comb` sensitivity list: the array
    /// WORD index (`mem[sel] = …`), the bit/part-select OFFSET (`r[idx] = …`,
    /// `mask[idx*8 +: 8] = …`), and any non-constant WIDTH. The written BASE net
    /// is NOT a read (writing `mask` does not make the block sensitive to
    /// `mask`). Symmetric with the `Signal { word }` / `Select` read arms — a
    /// block whose ONLY read of `idx` is through an LHS index otherwise never
    /// re-fires when `idx` changes (silent stale value; the V10 comb-sensitivity
    /// silent-wrong, §4.5.166).
    pub(crate) fn collect_lval_reads(
        &self,
        lhs: &ir::Lvalue,
        reads: &mut std::collections::BTreeSet<u32>,
    ) {
        for ch in &lhs.chunks {
            if let Some(w) = ch.word {
                self.collect_expr_reads(w, reads);
            }
            if let Some(o) = ch.offset {
                self.collect_expr_reads(o, reads);
            }
            if let Some(wd) = ch.width {
                self.collect_expr_reads(wd, reads);
            }
        }
    }
}
