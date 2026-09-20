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

    /// IEEE 1800 §27.4 versus §27.5 — how SOURCE TEXT may spell a generate scope,
    /// answered in one place because the storage key answers neither half.
    ///
    /// A generate-FOR block is an ARRAY of scopes: its hierarchical name carries an
    /// index at ANY trip count (`gl[0].x`), and the bare `gl.x` is not its name. A
    /// conditional / `case` / bare labelled block is a SINGLETON: its name is the bare
    /// label (`gi.x`), and `gi[0].x` is not. vita stores BOTH as `label[idx]`
    /// (`elaborate_gen_scoped` tags a singleton `label[0]` so `is_gen_scope_segment`
    /// recognizes it and `walk_scopes` resolves outer nets through it), so a one-trip
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
    /// TWO tests, because neither alone is the property: `gen_loop_labels` says the
    /// LRM made this name an array (a `for` block, at ANY trip count — the syntactic
    /// fact storage cannot recover), and the absent `[1]` says nothing else unrolled
    /// into it. The loop test has to come first: a one-trip loop passes the `[1]` test
    /// and must still be refused.
    pub(crate) fn singleton_scope_key(&self, level: &str, seg: &str) -> Option<String> {
        let label = if level.is_empty() {
            seg.to_string()
        } else {
            format!("{level}.{seg}")
        };
        if self.gen_loop_labels.contains(&label) {
            return None;
        }
        let g0 = format!("{label}[0]");
        let is_singleton = self.is_hier_scope(&g0) && !self.is_hier_scope(&format!("{label}[1]"));
        is_singleton.then_some(g0)
    }

    /// The first segment of an already-resolved storage `key` that the SOURCE spelled
    /// with a singleton generate scope's `[0]` storage index. `user_segs` is how many
    /// TRAILING segments of `key` the source wrote — the leading ones come from the
    /// reader's own prefix and are vita's spelling, never the design's, so they are not
    /// judged here.
    pub(crate) fn key_spells_indexed_singleton(
        &self,
        key: &str,
        user_segs: usize,
    ) -> Option<String> {
        let segs: Vec<&str> = key.split('.').collect();
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
    pub(crate) fn gen_spelling_hint(&self, prefix: &str, path: &[String]) -> Option<String> {
        let full = path.join(".");
        let head = path.first()?;
        if let Some(label) = Self::gen_seg_label(head) {
            if self
                .scoped_key_at_or_above(prefix, label, |k| self.gen_singleton_labels.contains(k))
                .is_some()
            {
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
        self.scoped_key_at_or_above(prefix, head, |k| self.gen_loop_labels.contains(k))
            .map(|_| {
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
        prefix: &str,
        path: &[String],
        fallback: String,
    ) {
        let msg = self.gen_spelling_hint(prefix, path).unwrap_or(fallback);
        self.error(MsgCode::ElabUnresolvedName, &msg);
    }
}
