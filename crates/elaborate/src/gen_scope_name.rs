//! IEEE 1800 §27.4 versus §27.5: how SOURCE TEXT may spell a generate scope.
//!
//! vita stores a generate-FOR iteration and a SINGLETON (conditional / `case` /
//! bare labelled) block under the SAME `label[idx]` shape, so a storage key cannot
//! say which spelling a design is allowed to write. These predicates are the one
//! home of that question; every hierarchical resolver consults them and nobody
//! re-derives the rule from a key.

use super::*;

impl Elaborator<'_> {
    /// The `label` of a scope segment spelled `label[idx]`, or `None` for a plain
    /// identifier segment. The synthetic `$func$g[0]$f` / `$itask$…` / `$blk$…`
    /// segments are excluded: their `[` belongs to a generate label they CARRY, not
    /// to the segment, and they never end in `]`.
    pub(crate) fn gen_seg_label(seg: &str) -> Option<&str> {
        let (label, idx) = seg.strip_suffix(']')?.rsplit_once('[')?;
        (!label.is_empty() && !idx.is_empty() && !label.starts_with('$')).then_some(label)
    }

    /// The first segment of an already-resolved storage `key` that the SOURCE spelled
    /// with a singleton generate scope's `[0]` storage index. `user_segs` is how many
    /// TRAILING segments of `key` the source wrote — the leading ones come from the
    /// reader's own prefix and are vita's spelling, never the design's, so they are not
    /// judged here.
    pub(crate) fn key_spells_indexed_singleton(
        &self,
        key: &DeclKey,
        user_segs: usize,
    ) -> Option<String> {
        let segs: Vec<&str> = found_text(key).split('.').collect();
        let start = segs.len().saturating_sub(user_segs);
        (start..segs.len()).find_map(|i| self.indexed_gen_singleton(&segs[..i].join("."), segs[i]))
    }

    /// The message a hierarchical reader emits when `path` names a generate scope the
    /// way vita STORES it instead of the way IEEE 1800 §27.4/§27.5 names it, so the
    /// refusal states the real reason rather than "no such net".
    ///
    /// LEADING segment only, and keyed on the label sets through
    /// `scoped_key_at_or_above` — the same outward walk `hier_resolve` commits its
    /// leading segment with. A deeper segment's enclosing scope is whatever that walk
    /// committed to, which this pure helper cannot re-derive without repeating the
    /// resolver, so a deeper offender keeps the generic unresolved-name message rather
    /// than risk naming the wrong scope.
    pub(crate) fn gen_spelling_hint(
        &self,
        prefix: &crate::ScopeRef,
        path: &[String],
    ) -> Option<String> {
        let full = path.join(".");
        let head = path.first()?;
        if let Some(label) = Self::gen_seg_label(head) {
            if self.walk_above_gen_singleton_label(prefix, label).is_some() {
                let mut fixed = path.to_vec();
                fixed[0] = label.to_string();
                return Some(format!(
                    "hierarchical name `{full}`: `{label}` is a conditional / `case` / \
                     labelled generate block, not a generate-for array — IEEE 1800 §27.5 \
                     gives it no index. Write `{}`.",
                    fixed.join(".")
                ));
            }
            return None;
        }
        self.walk_above_gen_loop_label(prefix, head).map(|_| {
            format!(
                "hierarchical name `{full}`: `{head}` is a generate-for block — IEEE 1800 \
                     §27.4 makes it an ARRAY of scopes, so the name needs an index \
                     (`{head}[0].…`)."
            )
        })
    }

    /// Report an unresolved hierarchical `path` read/written at `prefix`, stating the
    /// generate-scope SPELLING rule when that is the reason and `fallback` otherwise.
    /// The one place the two messages are chosen between, so every deferred lane says
    /// the same thing about the same name.
    pub(crate) fn error_hier_unresolved(
        &mut self,
        prefix: &crate::ScopeRef,
        path: &[String],
        fallback: String,
    ) {
        // ENG-1b: a clocking item that got no holder has no `cb.x` alias, so every
        // deferred lane reaching a use of it ends here — refuse it by name.
        if let Some(i) = self.clk_refused_at(prefix, path) {
            self.report_clk_refused(i, None, prefix);
            return;
        }
        let msg = self.gen_spelling_hint(prefix, path).unwrap_or(fallback);
        self.error(MsgCode::ElabUnresolvedName, &msg);
    }
}
