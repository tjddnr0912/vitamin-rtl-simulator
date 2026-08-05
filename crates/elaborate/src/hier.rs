//! hierarchical references — split out of the original `elaborate` lib.rs (mechanical move).

use super::*;
use crate::array_geom::IndexDomain;

/// A hierarchical part-select spec, deferred until the target net's range is known
/// (the offset is normalized against the net's LSB at resolution time).
#[derive(Clone, Copy)]
pub(crate) struct HierPart {
    pub(crate) raw_off: u32,
    pub(crate) width: u32,
    pub(crate) kind: ir::SelKind,
}

/// Does any `disable <label>` (single-segment) appear in this statement tree?
/// Drives LAZY exit-BB allocation for named blocks: a label nobody disables
/// lowers exactly like an unlabeled block (byte-identical CFG to the
/// pre-disable lowering — golden corpus unaffected). Fork children are
/// included (a child's cross-boundary disable is rejected loudly later;
/// scanning them keeps this a pure syntactic property). Task bodies are NOT
/// resolved here — `disable` of a caller's label from inside a task stays a
/// loud unsupported error (the label is then absent from the disable stack).
pub(crate) fn stmt_disables_label(s: &ast::Stmt, label: &str) -> bool {
    use ast::Stmt as S;
    match s {
        S::Disable { target, .. } => target.segments.len() == 1 && target.segments[0].name == label,
        S::Block { stmts, .. } | S::Fork { stmts, .. } => {
            stmts.iter().any(|st| stmt_disables_label(st, label))
        }
        S::If { then_s, else_s, .. } => {
            stmt_disables_label(then_s, label)
                || else_s
                    .as_deref()
                    .is_some_and(|e| stmt_disables_label(e, label))
        }
        S::Case { items, .. } => items.iter().any(|it| match it {
            ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => {
                stmt_disables_label(body, label)
            }
        }),
        S::For {
            init, step, body, ..
        } => {
            stmt_disables_label(init, label)
                || stmt_disables_label(step, label)
                || stmt_disables_label(body, label)
        }
        S::While { body, .. } | S::Repeat { body, .. } | S::Forever { body, .. } => {
            stmt_disables_label(body, label)
        }
        S::DelayCtrl { body, .. } | S::EventCtrl { body, .. } | S::Wait { body, .. } => body
            .as_deref()
            .is_some_and(|b| stmt_disables_label(b, label)),
        _ => false,
    }
}

/// §4.5.200: collect the TARGET task name of every HIERARCHICAL task enable
/// (`u1.tk(...)` — a `UserTaskCall` whose name has 2+ segments; the LAST segment is the
/// task name, the leading segments are the instance path) reachable from `s`. Recurses
/// through control flow, timing-wrapped bodies, blocks, and forks. Under-collection is
/// correct-or-loud — an un-collected hier enable simply stays loud instead of force-framing
/// its target. Companion to [`collect_callee_stmt`].
pub(crate) fn collect_hier_task_stmt(s: &ast::Stmt, out: &mut std::collections::BTreeSet<String>) {
    use ast::Stmt::*;
    match s {
        UserTaskCall { name, .. } if name.segments.len() >= 2 => {
            out.insert(name.segments.last().unwrap().name.clone());
        }
        If { then_s, else_s, .. } => {
            collect_hier_task_stmt(then_s, out);
            if let Some(e) = else_s {
                collect_hier_task_stmt(e, out);
            }
        }
        Case { items, .. } => {
            for it in items {
                match it {
                    ast::CaseItem::Match { body, .. } | ast::CaseItem::Default { body, .. } => {
                        collect_hier_task_stmt(body, out)
                    }
                }
            }
        }
        For {
            init, step, body, ..
        } => {
            collect_hier_task_stmt(init, out);
            collect_hier_task_stmt(step, out);
            collect_hier_task_stmt(body, out);
        }
        While { body, .. } | Repeat { body, .. } | Forever { body, .. } => {
            collect_hier_task_stmt(body, out)
        }
        Block { stmts, .. } | Fork { stmts, .. } => {
            for st in stmts {
                collect_hier_task_stmt(st, out);
            }
        }
        DelayCtrl { body, .. } | EventCtrl { body, .. } | Wait { body, .. } => {
            if let Some(b) = body {
                collect_hier_task_stmt(b, out);
            }
        }
        DeferredAssert { then_s, else_s, .. } => {
            collect_hier_task_stmt(then_s, out);
            collect_hier_task_stmt(else_s, out);
        }
        _ => {}
    }
}

impl Elaborator<'_> {
    /// Build a hierarchical unpacked-array element read from PRE-LOWERED index eids,
    /// mirroring [`Self::lower_array_read`] (first D indices → row-major flat word; one
    /// trailing index → a bit-select into the element). `None` (after a loud error) for
    /// a partial slice (`< D` indices) or a bit-of-bit (`> D+1`). The index eids were
    /// lowered at the read site (full param/genvar/formal context), so this only builds
    /// the flat-word arithmetic and the select.
    /// Build a hierarchical part-select READ (`dut.mem[i][m:l]`, scalar `dut.v[m:l]`,
    /// `[b+:w]`) — the READ twin of the `part` branch of
    /// [`Self::build_hier_sel_write_chunk`]. `idx_eids` (if any) select an
    /// unpacked-array ELEMENT word; the part-select then applies within it, with the
    /// offset normalized against the element/net LSB (so a non-zero-LSB net selects the
    /// right internal bits, and a `[N:0]` element is a no-op). `None` (after a loud
    /// error) for a multi-dim packed element sub-part-select (a deferred follow-on, as
    /// on the write side) or a stray index on a scalar.
    pub(crate) fn build_hier_read_part(
        &mut self,
        net: u32,
        idx_eids: &[u32],
        part: HierPart,
        path: &str,
    ) -> Option<u32> {
        // Ascending net (`logic [lo:hi]`) `+:`/`-:`: the select KIND was baked
        // descending at lowering (net direction unknown then), so it would walk the
        // wrong way and read partly out-of-range → silent-wrong. Loud-reject —
        // consistent with the ascending `[m:l]` part-select (already loud via the
        // descending-default width check). Ascending indexed hierarchical part-select
        // is a follow-on (a `[m:l]` or whole-net read works).
        if matches!(part.kind, ir::SelKind::PartIdxUp | ir::SelKind::PartIdxDown)
            && self.net_ascending(net)
        {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "an ascending-net indexed part-select (`[b+:w]`/`[b-:w]`) of hierarchical \
                     `{path}` is unsupported — read a `[m:l]` range or the whole net"
                ),
            );
            return None;
        }
        let word = if self.net_is_static_array(net) {
            let dims = self.net_dim_extents(net);
            let d = dims.len();
            if idx_eids.len() != d {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "a hierarchical part-select of unpacked array `{path}` must index \
                         every dimension to one element"
                    ),
                );
                return None;
            }
            Some(self.flatten_word_eids(&dims, idx_eids, &[], IndexDomain::ArrayWord))
        } else if self.packed_dims.contains_key(&net) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "a hierarchical part-select of multi-dim packed `{path}` is a follow-on \
                     (read whole elements `[i]` or the whole net)"
                ),
            );
            return None;
        } else {
            if !idx_eids.is_empty() {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!("too many indices in hierarchical part-select of `{path}`"),
                );
                return None;
            }
            None
        };
        let base = self.push_expr(ir::Expr::Signal { net, word });
        let offset = self.norm_offset_for_net(net, part.raw_off);
        Some(self.push_expr(ir::Expr::Select {
            base,
            offset,
            width: part.width,
            kind: part.kind,
        }))
    }

    pub(crate) fn build_hier_array_read(
        &mut self,
        net: u32,
        idx_eids: &[u32],
        path: &str,
    ) -> Option<u32> {
        let dims = self.net_dim_extents(net);
        let d = dims.len();
        if idx_eids.len() < d {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "a partial hierarchical slice of unpacked array `{path}` is unsupported \
                     (index every dimension)"
                ),
            );
            return None;
        }
        // An array element that is ITSELF a multi-dim PACKED net (`reg [3:0][7:0] qm [0:1]`
        // is BOTH `net_is_static_array` AND in `packed_dims`). The whole-element read
        // `dut.qm[i]` (exactly D unpacked indices) is supported and reads the full packed
        // word — but a further sub-index INTO the packed element (`dut.qm[i][j]`) needs a
        // packed-element bit-slice, NOT the single trailing bit-select below (which would
        // silently return 1 bit instead of iverilog's byte). The LOCAL array path
        // (`lower_array_read`/`collect_array_write`) shares this gap (it 1-bit-selects on
        // BOTH read and write), so rather than diverge — or silently mis-read — we
        // loud-reject the hierarchical sub-index as a deferred follow-on (restoring the
        // pre-follow-on loudness for this exact form; the whole-element read stays correct).
        if idx_eids.len() > d && self.packed_dims.contains_key(&net) {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "a hierarchical sub-element select into the packed element of array \
                     `{path}` is a deferred follow-on (read the whole element `[i]`, or \
                     index the packed element in its own scope)"
                ),
            );
            return None;
        }
        if idx_eids.len() > d + 1 {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "too many indices in hierarchical read of `{path}` (a multi-dim array \
                     element takes a single trailing bit-select)"
                ),
            );
            return None;
        }
        let word = self.flatten_word_eids(&dims, &idx_eids[..d], &[], IndexDomain::ArrayWord);
        let val = self.push_expr(ir::Expr::Signal {
            net,
            word: Some(word),
        });
        if let Some(&bidx) = idx_eids.get(d) {
            // Trailing bit-select into the element word — normalize against the
            // element's LSB (parity with the now-fixed `lower_array_read`); a `[N:0]`
            // element is a no-op. (P0-NZE.)
            let offset = self.norm_offset_for_net(net, bidx);
            let width = self.const_u32_expr(1, 32);
            return Some(self.push_expr(ir::Expr::Select {
                base: val,
                offset,
                width,
                kind: ir::SelKind::Bit,
            }));
        }
        Some(val)
    }

    /// Build a hierarchical PACKED multi-dim element read from PRE-LOWERED index eids,
    /// mirroring [`Self::lower_packed_read`] (first `k` indices → bit offset via
    /// `flatten_word_eids`; result width = product of the un-indexed inner dims → an
    /// indexed part-select). `None` (after a loud error) for over-indexing.
    pub(crate) fn build_hier_packed_read(
        &mut self,
        net: u32,
        idx_eids: &[u32],
        path: &str,
    ) -> Option<u32> {
        let dims = self.packed_dims[&net].clone();
        if idx_eids.len() > dims.len() {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "too many indices in hierarchical read of packed array `{path}` (more \
                     than its dimensions)"
                ),
            );
            return None;
        }
        let (ext, dirs) = Self::packed_split(&dims);
        let offset = self.flatten_word_eids(&ext, idx_eids, &dirs, IndexDomain::PackedElem);
        let elem_w: u64 = dims[idx_eids.len()..]
            .iter()
            .map(|&(_, w, _)| w as u64)
            .product();
        let base = self.push_expr(ir::Expr::Signal { net, word: None });
        let width = self.const_u32_expr(elem_w.min(u32::MAX as u64) as u32, 32);
        Some(self.push_expr(ir::Expr::Select {
            base,
            offset,
            width,
            kind: ir::SelKind::PartIdxUp,
        }))
    }

    /// Build the `LvalChunk` for a hierarchical element/bit-select OR part-select write
    /// from PRE-LOWERED index eids and the resolved net's shape, mirroring the local
    /// `collect_array_write` / `collect_packed_write` / part-select lvalue paths (driven
    /// by `flatten_word_eids`, preserving the write-site index context). A poison chunk
    /// (after a loud error) for an out-of-range index arity.
    pub(crate) fn build_hier_sel_write_chunk(
        &mut self,
        net: u32,
        idx_eids: &[u32],
        part: Option<HierPart>,
        path: &str,
    ) -> ir::LvalChunk {
        // HIER-REST-PS: a part-select write. `idx_eids` (if any) selects an array
        // ELEMENT word; the part-select then applies within it. A bare scalar/vector
        // base has no indices. The offset is normalized against the net's LSB here
        // (parity with the local part-select lvalue path).
        if let Some(p) = part {
            // Ascending net `+:`/`-:`: the select KIND was baked descending at
            // lowering (net direction unknown then) → it would write the wrong bits
            // (silent). Loud-reject, mirroring the read twin `build_hier_read_part`
            // and the already-loud ascending `[m:l]` write. (Read/write symmetric.)
            if matches!(p.kind, ir::SelKind::PartIdxUp | ir::SelKind::PartIdxDown)
                && self.net_ascending(net)
            {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "an ascending-net indexed part-select (`[b+:w]`/`[b-:w]`) WRITE of \
                         hierarchical `{path}` is unsupported — write a `[m:l]` range or the \
                         whole net"
                    ),
                );
                return poison_chunk();
            }
            let word = if self.net_is_static_array(net) {
                let dims = self.net_dim_extents(net);
                let d = dims.len();
                if idx_eids.len() != d {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "a hierarchical part-select of unpacked array `{path}` must index \
                             every dimension to one element"
                        ),
                    );
                    return poison_chunk();
                }
                Some(self.flatten_word_eids(&dims, idx_eids, &[], IndexDomain::ArrayWord))
            } else if self.packed_dims.contains_key(&net) {
                // a part-select on a bare multi-dim PACKED net selects whole outer
                // elements (N3.4) — a deferred follow-on for the hierarchical lane.
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "a hierarchical part-select of multi-dim packed `{path}` is a follow-on \
                         (write whole elements `[i]` or the whole net)"
                    ),
                );
                return poison_chunk();
            } else {
                if !idx_eids.is_empty() {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!("too many indices in hierarchical part-select of `{path}`"),
                    );
                    return poison_chunk();
                }
                None
            };
            let offset = self.norm_offset_for_net(net, p.raw_off);
            return ir::LvalChunk {
                net,
                word,
                offset: Some(offset),
                width: Some(p.width),
                kind: p.kind,
            };
        }
        if self.net_is_static_array(net) {
            let dims = self.net_dim_extents(net);
            let d = dims.len();
            if idx_eids.len() < d {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "a partial hierarchical slice of unpacked array `{path}` is unsupported \
                         (index every dimension)"
                    ),
                );
                return poison_chunk();
            }
            let word = self.flatten_word_eids(&dims, &idx_eids[..d], &[], IndexDomain::ArrayWord);
            let trailing = &idx_eids[d..];
            // Array-of-packed: trailing indices → an indexed part-select on the element.
            if !trailing.is_empty() {
                if let Some(pdims) = self.packed_dims.get(&net).cloned() {
                    if trailing.len() > pdims.len() {
                        self.error(
                            MsgCode::ElabUnsupported,
                            &format!(
                                "too many indices in hierarchical write of `{path}` (a multi-dim \
                                 array element takes a single trailing bit/part-select)"
                            ),
                        );
                        return poison_chunk();
                    }
                    let (ext, dirs) = Self::packed_split(&pdims);
                    let offset =
                        self.flatten_word_eids(&ext, trailing, &dirs, IndexDomain::PackedElem);
                    let elem_w: u64 = pdims[trailing.len()..]
                        .iter()
                        .map(|&(_, w, _)| w as u64)
                        .product();
                    let width = self.const_u32_expr(elem_w.min(u32::MAX as u64) as u32, 32);
                    return ir::LvalChunk {
                        net,
                        word: Some(word),
                        offset: Some(offset),
                        width: Some(width),
                        kind: ir::SelKind::PartIdxUp,
                    };
                }
            }
            // Plain (flat-vector) element: whole word (0 trailing) or single bit (1).
            let (offset, width) = match trailing.len() {
                0 => (None, None),
                1 => {
                    // P0-NZE: normalize the trailing bit against the element's LSB
                    // (a `[N:0]` element is a no-op).
                    let off = self.norm_offset_for_net(net, trailing[0]);
                    (Some(off), Some(self.const_u32_expr(1, 32)))
                }
                _ => {
                    self.error(
                        MsgCode::ElabUnsupported,
                        &format!(
                            "too many indices in hierarchical write of `{path}` (a single trailing \
                             bit-select)"
                        ),
                    );
                    return poison_chunk();
                }
            };
            return ir::LvalChunk {
                net,
                word: Some(word),
                offset,
                width,
                kind: ir::SelKind::Bit,
            };
        }
        if self.packed_dims.contains_key(&net) {
            let dims = self.packed_dims[&net].clone();
            if idx_eids.len() > dims.len() {
                self.error(
                    MsgCode::ElabUnsupported,
                    &format!(
                        "too many indices in hierarchical write of packed array `{path}` (more \
                         than its dimensions)"
                    ),
                );
                return poison_chunk();
            }
            let (ext, dirs) = Self::packed_split(&dims);
            let offset = self.flatten_word_eids(&ext, idx_eids, &dirs, IndexDomain::PackedElem);
            let elem_w: u64 = dims[idx_eids.len()..]
                .iter()
                .map(|&(_, w, _)| w as u64)
                .product();
            let width = self.const_u32_expr(elem_w.min(u32::MAX as u64) as u32, 32);
            return ir::LvalChunk {
                net,
                word: None,
                offset: Some(offset),
                width: Some(width),
                kind: ir::SelKind::PartIdxUp,
            };
        }
        // scalar / vector → a single bit-select.
        if idx_eids.len() != 1 {
            self.error(
                MsgCode::ElabUnsupported,
                &format!(
                    "too many indices in hierarchical write of `{path}` (a scalar/vector takes a \
                     single bit-select)"
                ),
            );
            return poison_chunk();
        }
        let offset = self.norm_offset_for_net(net, idx_eids[0]);
        let width = self.const_u32_expr(1, 32);
        ir::LvalChunk {
            net,
            word: None,
            offset: Some(offset),
            width: Some(width),
            kind: ir::SelKind::Bit,
        }
    }

    /// Resolve a dotted hierarchical `path` (length ≥ 2) from scope `prefix` per IEEE
    /// 1800 §23.6 upward name resolution: resolve the LEADING segment to a scope,
    /// COMMITTING to the innermost enclosing scope where it is found, then resolve the
    /// remaining path WITHIN that scope. Crucially, once the leading segment is found —
    /// as a child scope, a singleton generate block (`g` ⇒ `g[0]`), or a NET — the
    /// search STOPS: a missing remainder there is unresolved, NOT a reason to keep
    /// walking outward (review N3 HIGH: the old whole-tail outward strip silently
    /// grabbed an unrelated outer net when the leading segment matched an inner scope
    /// whose remainder was invalid, e.g. `b.v` / a local-shadowed `cfg.mode`).
    pub(crate) fn hier_lookup(&self, prefix: &str, path: &[String]) -> Option<u32> {
        self.hier_resolve(prefix, path, &self.symbols)
    }

    /// Hierarchical READ of a cross-instance PARAMETER (`dut.WIDTH`): the same
    /// §23.6 commit-to-scope walk as [`Self::hier_lookup`] but against the
    /// persistent `hier_params` table. (Params are restored out of `self.params`
    /// after each instance, so a post-elaboration read needs the persistent copy.)
    /// Folded only in a RUNTIME/expression context — a hierarchical param in a
    /// CONSTANT context (a width / `localparam`) is loud, since the sibling
    /// instance is not yet elaborated when the const-eval needs the value.
    pub(crate) fn hier_lookup_param(&self, prefix: &str, path: &[String]) -> Option<i64> {
        self.hier_resolve(prefix, path, &self.hier_params)
    }

    /// The `(declared width, signed)` of a hierarchical param read (`dut.W`), when
    /// the param has a determinate declared width — so the cross-instance const is
    /// materialized at THAT width, not the value-inferred 32 bits (mirrors the
    /// bare-param read path). `param_meta` is persistent (never restored), so it is
    /// visible after the sibling instance has bound its params.
    pub(crate) fn hier_lookup_param_meta(
        &self,
        prefix: &str,
        path: &[String],
    ) -> Option<(u32, bool)> {
        self.hier_resolve(prefix, path, &self.param_meta)
    }

    /// Shared commit-to-scope resolution for a dotted hierarchical `path`
    /// (length ≥ 2), generic over the leaf `table` (`symbols` for nets,
    /// `hier_params` for parameters). Resolve the LEADING segment to a scope,
    /// COMMITTING to the innermost enclosing scope where it is found, then resolve
    /// the remaining path WITHIN that scope. Once the leading segment is found — a
    /// child scope, a singleton generate block (`g` ⇒ `g[0]`), or a NET leaf — the
    /// search STOPS: a missing remainder is unresolved, NOT a reason to keep
    /// walking outward (review N3 HIGH: the old whole-tail outward strip silently
    /// grabbed an unrelated outer net when the leading segment matched an inner
    /// scope whose remainder was invalid, e.g. `b.v` / a local-shadowed `cfg.mode`).
    pub(crate) fn hier_resolve<V: Copy>(
        &self,
        prefix: &str,
        path: &[String],
        table: &std::collections::BTreeMap<String, V>,
    ) -> Option<V> {
        // A2b (adversarial sound #2): a hierarchical reference FROM package
        // scope (a `$pkg$…` prefix = the package pre-sweep, the only proc
        // context there) is LRM-illegal (§26.3 — packages see only their own
        // scope) and has no legit resolution target (no interface/class state
        // can exist in a package). Committed-unresolved → every deferred
        // resolver stays loud, never an oracle-divergent silent value.
        if prefix.starts_with("$pkg$") {
            return None;
        }
        let first = &path[0];
        let tail = path.join(".");
        let rest = path[1..].join(".");
        let mut segs: Vec<&str> = if prefix.is_empty() {
            Vec::new()
        } else {
            prefix.split('.').collect()
        };
        loop {
            let level = segs.join("."); // "" at the outermost (absolute) level
            let base = if level.is_empty() {
                first.clone()
            } else {
                format!("{level}.{first}")
            };
            // (a) leading segment names a child SCOPE (module/genblock instance) here.
            if self.is_hier_scope(&base) {
                let full = if level.is_empty() {
                    tail.clone()
                } else {
                    format!("{level}.{tail}")
                };
                // A2b-prereq (adversarial diff F2): a package-variable IMPORT
                // alias is a LEXICAL binding only (IEEE §26.3 — an import is
                // not a declaration in the importing scope), so a HIERARCHICAL
                // path must never resolve through it (iverilog: "Unable to
                // bind"). Committed-unresolved → the caller stays loud.
                if self.pkg_var_aliases.contains_key(&full) {
                    return None;
                }
                return table.get(&full).copied(); // committed: Some=hit, None=unresolved
            }
            // (b) SINGLETON generate block: vita names a named `if`/`begin` block `g[0]`,
            // but the hierarchical name is the bare `g` (IEEE: the implicit [0] is not
            // part of the name). Map only the leading segment.
            let base0 = format!("{base}[0]");
            if self.is_hier_scope(&base0) {
                let full = if rest.is_empty() {
                    base0
                } else {
                    format!("{base0}.{rest}")
                };
                if self.pkg_var_aliases.contains_key(&full) {
                    return None; // same §26.3 rule as (a)
                }
                return table.get(&full).copied();
            }
            // (c) leading segment names a NET (not a scope) here: `.member` on a plain
            // net is unsupported (committed → unresolved, loud).
            if self.symbols.contains_key(&base) {
                return None;
            }
            // not found at this level → walk one scope outward.
            if segs.is_empty() {
                return None;
            }
            segs.pop();
        }
    }

    /// True iff `base` is a hierarchical SCOPE — some net OR parameter is named
    /// `base.<…>`. Used by `hier_resolve` to commit the leading path segment to a
    /// scope (instance/genblock); the `hier_params` arm lets a param-only child
    /// module (no nets) still register as a scope.
    pub(crate) fn is_hier_scope(&self, base: &str) -> bool {
        let probe = format!("{base}.");
        let hit = |k: &String| k.starts_with(&probe);
        self.symbols
            .range(probe.clone()..)
            .next()
            .is_some_and(|(k, _)| hit(k))
            || self
                .hier_params
                .range(probe.clone()..)
                .next()
                .is_some_and(|(k, _)| hit(k))
    }

    /// Child prefix = current prefix + child instance name.
    pub(crate) fn child_prefix(&self, inst_name: &str) -> String {
        if self.cur_prefix.is_empty() {
            inst_name.to_string()
        } else {
            format!("{}.{}", self.cur_prefix, inst_name)
        }
    }

    // ── multi-dim unpacked-array access (read/write, (a)-flattening) ─────────
    //
    // A `base[i0][i1]…[ik]` selection parses as a left-nested BitSelect chain. If
    // the innermost base is a plain single-segment `Ident` resolving to an ARRAY
    // net (`array_len > 1`), the whole chain is an array access; otherwise it is an
    // ordinary bit/part-select on a scalar value and these helpers return `None` so
    // the caller's existing logic runs. The index `Vec` is returned in SOURCE order
    // (`[i0, i1, …, ik]`): the chain walk yields outer-first, so the base part is
    // reversed and the outermost index appended last.

    /// Collect a read-side `base[index]` chain rooted at an array `Ident`.
    /// Walk a read-context BitSelect chain `base[i]…[k]` that bottoms out at a
    /// TWO-segment hierarchical Ident (`dut.grid`) whose dotted name is NOT already a
    /// known net (an interface-member alias keeps the normal path). Returns the path
    /// segments and the indices in SOURCE order (`dut.grid[i][j]` → `[i, j]`). This is
    /// the hierarchical twin of [`Self::expr_array_chain`]/[`Self::expr_packed_chain`]
    /// — but it stops at a 2-seg Ident (cross-instance) instead of a 1-seg local net,
    /// and it does NOT yet know the net's shape (resolved post-elaboration), so it only
    /// captures the path + index ASTs for the deferred fixup. `None` if the chain does
    /// not bottom at such a hierarchical base.
    pub(crate) fn hier_sel_chain<'a>(
        &self,
        base: &'a ast::Expr,
        index: &'a ast::Expr,
    ) -> Option<(Vec<String>, Vec<&'a ast::Expr>)> {
        let mut outer_first: Vec<&ast::Expr> = Vec::new();
        let mut cur = base;
        let path = loop {
            match &cur.kind {
                ast::ExprKind::BitSelect { base: b, index: i } => {
                    outer_first.push(i);
                    cur = b;
                }
                // HIER-REST①: a 2-segment ref (`dut.mem[i]`) OR a deeper one
                // (`m.l.mem[i]`) — any multi-segment cross-instance hierarchical base.
                ast::ExprKind::Ident(p) if p.segments.len() >= 2 => {
                    let joined = p
                        .segments
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join(".");
                    // A known dotted symbol (interface member) is a real net — not a
                    // cross-instance hierarchical ref. Let the normal path handle it.
                    if self.lookup_net_scoped(&joined).is_some() {
                        return None;
                    }
                    break p
                        .segments
                        .iter()
                        .map(|s| s.name.clone())
                        .collect::<Vec<_>>();
                }
                _ => return None,
            }
        };
        outer_first.reverse(); // base-chain → source order
        outer_first.push(index); // outermost index is last in source order
        Some((path, outer_first))
    }

    /// Expression twin of [`Self::lval_hier_chain`]: given a PART-select `base`, walk
    /// nested BitSelects (`dut.mem[i]`) or a bare multi-segment ref (`dut.v`) down to a
    /// cross-instance hierarchical Ident, returning the dotted path and element indices
    /// (source order). `None` for a single-segment local base or a known dotted
    /// interface-member alias (both handled by the normal part-select path). Used to
    /// DEFER a hierarchical part-select READ (`dut.mem[i][m:l]`, `dut.v[m:l]`) whose net
    /// range — and thus the LSB offset normalization — is unknown until pass 8.
    pub(crate) fn hier_chain<'a>(
        &self,
        base: &'a ast::Expr,
    ) -> Option<(Vec<String>, Vec<&'a ast::Expr>)> {
        let mut outer_first: Vec<&ast::Expr> = Vec::new();
        let mut cur = base;
        let path = loop {
            match &cur.kind {
                ast::ExprKind::BitSelect { base: b, index: i } => {
                    outer_first.push(i);
                    cur = b;
                }
                ast::ExprKind::Ident(p) if p.segments.len() >= 2 => {
                    let joined = p
                        .segments
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join(".");
                    if self.lookup_net_scoped(&joined).is_some() {
                        return None; // interface-member alias — normal path
                    }
                    break p
                        .segments
                        .iter()
                        .map(|s| s.name.clone())
                        .collect::<Vec<_>>();
                }
                _ => return None,
            }
        };
        outer_first.reverse(); // base-chain → source order
        Some((path, outer_first))
    }

    /// Core of the lvalue hierarchical-chain detectors: walk an lvalue `base` of
    /// nested BitSelects down to a multi-segment cross-instance hierarchical Ident,
    /// returning the dotted path and the element indices (source order). A
    /// single-segment local lvalue or a known dotted interface-member alias returns
    /// None (handled by the normal lvalue path).
    pub(crate) fn lval_hier_chain<'a>(
        &self,
        base: &'a ast::Lvalue,
    ) -> Option<(Vec<String>, Vec<&'a ast::Expr>)> {
        let mut outer_first: Vec<&ast::Expr> = Vec::new();
        let mut cur = base;
        let path = loop {
            match cur {
                ast::Lvalue::BitSelect {
                    base: b, index: i, ..
                } => {
                    outer_first.push(i);
                    cur = b;
                }
                ast::Lvalue::Ident(p) if p.segments.len() >= 2 => {
                    let joined = p
                        .segments
                        .iter()
                        .map(|s| s.name.as_str())
                        .collect::<Vec<_>>()
                        .join(".");
                    if self.lookup_net_scoped(&joined).is_some() {
                        return None; // interface-member alias — normal path
                    }
                    break p
                        .segments
                        .iter()
                        .map(|s| s.name.clone())
                        .collect::<Vec<_>>();
                }
                _ => return None,
            }
        };
        outer_first.reverse(); // base-chain → source order
        Some((path, outer_first))
    }

    /// LVALUE twin of [`Self::hier_sel_chain`]: `Some((path, idxs))` when an lvalue
    /// `base[i]…[k]` bottoms out at a multi-segment cross-instance hierarchical ref
    /// (`dut.mem[i] = …`, `m.l.v[3] <= …`).
    pub(crate) fn lval_hier_sel_chain<'a>(
        &self,
        base: &'a ast::Lvalue,
        index: &'a ast::Expr,
    ) -> Option<(Vec<String>, Vec<&'a ast::Expr>)> {
        let (path, mut idxs) = self.lval_hier_chain(base)?;
        idxs.push(index); // outermost index is last in source order
        Some((path, idxs))
    }

    /// §4.5.166 HIER twin: after the deferred hierarchical indexed read/write
    /// resolvers have patched real net + index eids into the stmt/expr arenas,
    /// recompute the read-set of every comb-inferred process (recorded in
    /// `comb_inferred_procs` — bare self-timed `always` blocks are excluded, so
    /// their intentionally-empty sensitivity is never widened). A hierarchical
    /// index (`y = dut.mem[idx]` / `dut.mem[idx] = v`) is behind a sentinel at
    /// lowering time and thus dropped from the original inference; the patched
    /// body now exposes it. The recomputed set only WIDENS (local reads are
    /// unchanged; the newly-visible hier index is added) — never narrows a real
    /// read — so non-hier comb blocks are byte-identical. Two-phase to satisfy
    /// the borrow checker: gather under `&self`, then apply under `&mut self`.
    pub(crate) fn recompute_comb_sensitivity_after_hier(&mut self) {
        let mut updated: Vec<(u32, Vec<u32>)> = Vec::new();
        for &pid in &self.comb_inferred_procs {
            let nets = self.comb_read_set(&self.processes[pid as usize].body);
            updated.push((pid, nets));
        }
        for (pid, nets) in updated {
            self.processes[pid as usize].sensitivity.edges = nets
                .into_iter()
                .map(|net| ir::EdgeTerm {
                    net,
                    kind: ir::EdgeKind::AnyEdge,
                })
                .collect();
        }
    }
}
