//! The hierarchical resolver (§5.2 Q-DECLKEY DK1-5a): the IEEE 1800 §23.6 commit-to-scope walk of a
//! dotted path (`hier_resolve`), the storage key it builds inside the committed scope
//! (`hier_key_within`), and two of the §27.4 / §27.5 generate-spelling helpers they ask
//! (`indexed_gen_singleton`, `singleton_scope_key`), with the scope question the walk asks
//! (`is_hier_scope`, X16, private) — moved here whole from `hier.rs` and
//! `gen_scope_name.rs`. They are Elaborator methods (tier E), so every caller's tokens are unchanged ;
//! a carried scope's text is read here through the module's own [`super::Scopes::scope_text`], private
//! since this unit, so no bridge is left for them outside.
//!
//! WHAT LEAVES AS TEXT, AND UNTIL WHEN:
//! - `indexed_gen_singleton`'s return, the label key `level.label` of a singleton generate scope:
//!   LEAVE L12, read by `key_spells_indexed_singleton` (`gen_scope_name.rs`) ; closes at DK3-b, where that
//!   reader becomes an entry of this module and this fn goes private.
//! - `hier_resolve`'s `table` is the caller's text-keyed map (`&BTreeMap<String, V>`, reached through
//!   window w9), and eleven store reads here go through the window by text, inside the module, until
//!   DK3-b deletes it: `gen_singleton_labels.contains` in `indexed_gen_singleton` and in
//!   `singleton_scope_key` (2) ; `pkg_var_aliases.contains_key` x2 and `symbols.contains_key` in
//!   `hier_resolve` (3) ; `symbols` / `hier_params` / `wide_param_bits` `.contains_key` in
//!   `hier_key_within` (3) ; and the same three stores' `.range` scans in `is_hier_scope` (3).
//! - ONE call hands text to a fn outside the module: `Self::gen_seg_label(seg)` in
//!   `indexed_gen_singleton` passes its own entered `seg` through (no key or scope text is formed before
//!   it). It is NOT pinned by class until DK1-6 (the outward-call table SURF_OUTCALL): until then a
//!   second such call is caught only as a new OUT-C row and a BODY re-pin of the scope surface.
//! - `hier_key_within`, `singleton_scope_key` and `is_hier_scope` are private: their text returns and
//!   parameters stay inside.

use crate::Elaborator;

impl Elaborator<'_> {
    /// IEEE 1800 §27.4 versus §27.5 — how SOURCE TEXT may spell a generate scope,
    /// answered in one place because the storage key answers neither half.
    ///
    /// A generate-FOR block is an ARRAY of scopes: its hierarchical name carries an
    /// index at ANY trip count (`gl[0].x`), and the bare `gl.x` is not its name. A
    /// conditional / `case` / bare labelled block is a SINGLETON: its name is the bare
    /// label (`gi.x`), and `gi[0].x` is not. vita stores BOTH as `label[idx]`
    /// (`elaborate_gen_scoped` tags a singleton `label[0]` so `is_gen_scope_segment`
    /// recognizes it and `walk_scopes_key` resolves outer nets through it), so a one-trip
    /// loop and a conditional block leave exactly the same keys — only the two label
    /// sets separate them.
    ///
    /// `level` is the fully-qualified scope the segment is resolved IN; `seg` is the
    /// segment exactly as the source wrote it. `Some(label key)` means this spelling is
    /// not the scope's name, so the resolver declines and the reader stays loud
    /// (MEASURED: `gi[0].x` read `3` at exit 0 where iverilog refuses to bind
    /// `gi['sd0].x` and verilator cannot find `gi[0]` — net read and write, localparam,
    /// `$bits`, event control, element and part select, cross-instance, nested, and an
    /// unnamed `genblk1`).
    pub(crate) fn indexed_gen_singleton(&self, level: &str, seg: &str) -> Option<String> {
        let label = Self::gen_seg_label(seg)?;
        let key = if level.is_empty() {
            label.to_string()
        } else {
            format!("{level}.{label}")
        };
        self.gen_singleton_labels.contains(&key).then_some(key)
    }

    /// The `[0]` STORAGE key of the SINGLETON generate scope a BARE label names in
    /// source text (`g.x` ⇒ `g[0].x`), or `None` when `level.seg` is not one.
    ///
    /// Keyed POSITIVELY on `gen_singleton_labels`, the same set `display_prefix`
    /// (`generate.rs`) and [`Self::indexed_gen_singleton`] ask, because only the
    /// ELABORATOR knows which construct minted a `label[0]` key — storage cannot
    /// recover it. The first draft asked the question negatively (`label` is not in
    /// `gen_loop_labels`, `label[0]` is a scope and `label[1]` is not), which is a
    /// different question: an INSTANCE-ARRAY label is in neither label set, so a
    /// one-element array `ch u [0:0] ();` satisfied both halves and the bare `u.q`
    /// resolved to element 0 (MEASURED: `A=7` at exit 0, where iverilog says
    /// "error: Unable to bind wire/reg/memory `u.q' in `g408'" and verilator says
    /// "%Error: Can't find definition of 'u'" — an element must be spelled `u[0].q`;
    /// the same three-tool split on the ANSI-ported `ch u [0:0] (.p(w));` and on a
    /// `module ch();` child). `generate.rs:51-58` had already recorded this exact
    /// hazard for the `%m` twin.
    ///
    /// The `[0]` scope test stays: `hier_key_within` documents the fallback as
    /// "only when the `[0]` spelling is a REAL scope", so an empty labelled block
    /// keeps handing back the plain spelling.
    fn singleton_scope_key(&self, level: &str, seg: &str) -> Option<String> {
        let label = if level.is_empty() {
            seg.to_string()
        } else {
            format!("{level}.{seg}")
        };
        if !self.gen_singleton_labels.contains(&label) {
            return None;
        }
        let g0 = format!("{label}[0]");
        self.is_hier_scope(&g0).then_some(g0)
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
    pub(crate) fn hier_resolve<V: Clone>(
        &self,
        prefix: &crate::ScopeRef,
        path: &[String],
        table: &std::collections::BTreeMap<String, V>,
    ) -> Option<V> {
        // A2b (adversarial sound #2): a hierarchical reference FROM package
        // scope (a `$pkg$…` prefix = the package pre-sweep, the only proc
        // context there) is LRM-illegal (§26.3 — packages see only their own
        // scope) and has no legit resolution target (no interface/class state
        // can exist in a package). Committed-unresolved → every deferred
        // resolver stays loud, never an oracle-divergent silent value.
        if self.lex.scope_is_pkg(prefix) {
            return None;
        }
        let prefix = self.lex.scope_text(prefix);
        let first = &path[0];
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
            // (a0) IEEE 1800 §27.5 (`indexed_gen_singleton`): `gi[0]` is how vita
            // STORES a conditional / `case` / bare labelled generate block, never how
            // a design may name it — both oracles refuse to bind `gi[0].x`. Committed-
            // unresolved, ahead of every arm below, so the reader stays loud instead
            // of reaching the scope through its storage spelling.
            if self.indexed_gen_singleton(&level, first).is_some() {
                return None;
            }
            // (a) leading segment names a child SCOPE (module/genblock instance) here.
            if self.is_hier_scope(&base) {
                let Some(full) = self.hier_key_within(&base, &path[1..]) else {
                    return None; // committed, remainder unresolvable
                };
                // A2b-prereq (adversarial diff F2): a package-variable IMPORT
                // alias is a LEXICAL binding only (IEEE §26.3 — an import is
                // not a declaration in the importing scope), so a HIERARCHICAL
                // path must never resolve through it (iverilog: "Unable to
                // bind"). Committed-unresolved → the caller stays loud.
                if self.pkg_var_aliases.contains_key(&full) {
                    return None;
                }
                return table.get(&full).cloned(); // committed: Some=hit, None=unresolved
            }
            // (b) SINGLETON generate block: vita names a named `if`/`begin` block `g[0]`,
            // but the hierarchical name is the bare `g` (IEEE: the implicit [0] is not
            // part of the name).
            let base0 = format!("{base}[0]");
            if self.is_hier_scope(&base0) {
                // §27.4: a generate-FOR label is an ARRAY of scopes at ANY trip count,
                // so the bare spelling is not its name either — `singleton_scope_key`
                // is the same test `hier_key_within` applies one dot further in, and
                // this leading position was the one place that did not ask it
                // (MEASURED: `gl.x` on a two-trip loop read iteration 0's `5` at exit 0
                // where both oracles refuse). The label IS bound at this level, so a
                // failure commits rather than walking outward.
                let base0 = self.singleton_scope_key(&level, first)?;
                let full = if rest.is_empty() {
                    base0
                } else {
                    self.hier_key_within(&base0, &path[1..])?
                };
                if self.pkg_var_aliases.contains_key(&full) {
                    return None; // same §26.3 rule as (a)
                }
                return table.get(&full).cloned();
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

    /// Build the STORAGE key for `rest` inside the already-committed scope `base`,
    /// mapping each intermediate segment that names a SINGLETON generate block to
    /// the `seg[0]` spelling vita stores it under.
    ///
    /// ⚠️ This exists because the `g` ⇒ `g[0]` mapping used to be applied to the
    /// LEADING segment only — the comment on arm (b) said so in as many words
    /// ("Map only the leading segment"). That is fine for a same-module reference,
    /// where the block IS the leading segment (`gblk.x`), and it is exactly wrong
    /// one dot further out: `u.gblk.x` commits `u` in arm (a) and then looked up
    /// the literal `u.gblk.x`, while the net is stored as `u.gblk[0].x`. MEASURED:
    /// 19 cells across `if` / `if…else` / `case` / bare `begin : g`, read and
    /// write, net / localparam / instance-inside, depth 1 and 2 — all E3010 in
    /// vita and all correct in iverilog. The indexed spelling (`u.gblk[0].x`)
    /// already worked, which is why a suite that pinned the local spelling
    /// (`hier_ref.rs::named_generate_block_read`, green since the initial commit)
    /// left the whole cross-instance family invisible.
    ///
    /// PLAIN FIRST, `[0]` ONLY AS A FALLBACK, and only when the `[0]` spelling is a
    /// REAL scope: a module may legitimately hold an instance named `g` (measured:
    /// `ub.g.r` on `module b; sub g(); endmodule` resolves and must keep
    /// resolving), and an instance and a generate block cannot share a name in one
    /// scope, so the fallback can never steal a name that exists. A segment that is
    /// neither keeps the committed-unresolved contract — `None`, never a walk
    /// further outward (review N3 HIGH).
    ///
    /// ⚠️⚠️ SINGLETON ONLY, and this guard is not decoration — the first draft
    /// omitted it and the census caught the regression it caused. `g` on a
    /// generate-FOR with two iterations is an ambiguous name that iverilog REFUSES
    /// to compile; without the singleton test the fallback answered it with
    /// iteration 0's net (`u.g.x` → `a5`, exit 0), turning a correct loud refusal
    /// into a silent pick — the one trade the accuracy ladder forbids outright.
    /// `seg[1]` existing is the whole discriminator: a conditional/`case`/bare
    /// block is a singleton by construction, an unrolled loop is not. That test now
    /// lives in [`Self::singleton_scope_key`] so `hier_resolve`'s leading-segment arm
    /// asks it too, and its `[0]`-spelled mirror in [`Self::indexed_gen_singleton`].
    fn hier_key_within(&self, base: &str, rest: &[String]) -> Option<String> {
        let mut cur = base.to_string();
        for (i, seg) in rest.iter().enumerate() {
            // §27.5, at every depth: a singleton generate scope named with its `[0]`
            // storage index (`u.gi[0].x`) is not a name the design may write.
            if self.indexed_gen_singleton(&cur, seg).is_some() {
                return None;
            }
            let plain = format!("{cur}.{seg}");
            if i + 1 == rest.len() {
                // LEAF: the caller's `table` decides whether this is a hit, so hand
                // back the plain spelling unless only the `[0]` scope spelling
                // exists (a whole-scope reference, e.g. a genblock-typed leaf).
                if self.symbols.contains_key(&plain)
                    || self.hier_params.contains_key(&plain)
                    || self.wide_param_bits.contains_key(&plain)
                {
                    return Some(plain);
                }
                return Some(self.singleton_scope_key(&cur, seg).unwrap_or(plain));
            }
            if self.is_hier_scope(&plain) {
                cur = plain;
                continue;
            }
            cur = self.singleton_scope_key(&cur, seg)?;
        }
        Some(cur)
    }

    /// True iff `base` is a hierarchical SCOPE — some net OR parameter is named
    /// `base.<…>`. Used by `hier_resolve` to commit the leading path segment to a
    /// scope (instance/genblock); the `hier_params` arm lets a param-only child
    /// module (no nets) still register as a scope.
    ///
    /// ⚠️ A child that contributes NO key to either table — an empty module, or one
    /// whose only declaration is a `string`, a >64-bit, or a non-integral `real`
    /// parameter — is not a scope, so a path through it walks OUTWARD and can commit
    /// to an unrelated same-named instance in an enclosing scope (measured: an inner
    /// `u.P` answered the OUTER `u`'s `P`). Widening this by table is not the fix —
    /// the honest predicate is "is `base` an instance path", which `instances_info`
    /// already knows. Recorded in ROADMAP §2 with the measured cases.
    fn is_hier_scope(&self, base: &str) -> bool {
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
            // A child whose only parameter is wider than i64 lives in the wide side
            // map alone; without this arm it was not a scope at all.
            || self
                .wide_param_bits
                .range(probe.clone()..)
                .next()
                .is_some_and(|(k, _)| hit(k))
    }
}
