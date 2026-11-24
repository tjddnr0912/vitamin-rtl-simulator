//! The outward scope walk (§5.2 Q-DECLKEY DK1-4a): the key-returning walk of a name from the current
//! scope (or a carried one) outward through the transparent generate / `$func$` / `$itask$` / `$blk$`
//! scopes, its saved-prefix forms, the package window's probe it asks first, and the generate-segment
//! test it stops on — moved here whole from `scope.rs`, `generate.rs` and `decl_scope.rs` (the value
//! form over a store, `walk_scopes`, stays in `scope.rs`: a caller of the walk, typed at DK1-4b). They
//! are Elaborator methods (tier E), so every caller's tokens are unchanged ; inside, they read the
//! current scope's text and a carried scope's text as this module's own fields, not through the
//! bridges. Their text signatures stay until DK1-4b (the found key's type): the walk's return
//! (`Option<String>`) and the text its `hit` callback is handed are LEAVE rows closing there.

use crate::decl_scope::{DeclArm, DeclPhase};
use crate::Elaborator;

impl Elaborator<'_> {
    #[inline]
    pub(crate) fn walk_scopes_key(&self, name: &str, hit: impl Fn(&str) -> bool) -> Option<String> {
        self.walk_scopes_key_inner(name, hit, false)
    }

    /// [`Self::walk_scopes_key`]'s shape for a SAVED prefix: the innermost key
    /// `<scope>.<name>` that satisfies `hit`, searching `prefix` and every scope
    /// enclosing it.
    ///
    /// The deferred hierarchical resolvers run after `cur_prefix` has moved on, so they
    /// carry the prefix that was live when the reference was LOWERED and cannot use the
    /// ambient walk. Diagnostic-only today (V33-3's enum-label receiver test).
    #[inline]
    pub(crate) fn scoped_key_at_or_above(
        &self,
        prefix: &crate::ScopeRef,
        name: &str,
        hit: impl Fn(&str) -> bool,
    ) -> Option<String> {
        let prefix = self.lex.names[prefix.0 .0 as usize].as_str();
        let mut scope = prefix;
        loop {
            let key = if scope.is_empty() {
                name.to_string()
            } else {
                format!("{scope}.{name}")
            };
            if hit(&key) {
                return Some(key);
            }
            match scope.rfind('.') {
                Some(i) => scope = &scope[..i],
                None if scope.is_empty() => return None,
                None => scope = "",
            }
        }
    }

    /// r19: [`Self::walk_scopes_key`], but the outward walk STOPS as soon as a scope
    /// level binds `name` as a NET without `hit` matching there.
    ///
    /// The plain walk is innermost-wins only WITHIN the single map `hit` probes. A
    /// function/task/block/generate local is a net (in `symbols`), so a consumer that
    /// probes a SIDE-map (`string_array_elems`) sailed straight past an inner local
    /// and resolved it to an OUTER side-map entry — silently, and for non-string
    /// locals too: a task-local `logic [7:0] sa[2]` write landed on a module `string
    /// sa[2]` element, and a generate-local `logic [15:0] sa` read was even
    /// range-checked against the outer array's declared `[0:1]`.
    ///
    /// Deliberately NOT the default. The shadow test keys on `symbols`, which is
    /// populated DURING elaboration, so it is only sound where the lookup cannot run
    /// before the shadowing net exists. `elaborate_gen_item` re-folds every generate
    /// control expression once per phase but reports a fold failure only in the Nets
    /// phase, so making the param lookup (`lookup_scoped`) shadow-aware let a nested
    /// generate fold in Nets and fail in Logic — silently deleting the whole generate
    /// body, exit 0. The `params`/`param_meta` consumers therefore keep the plain
    /// walk; that leaves an inner net failing to shadow an outer param (ROADMAP §2,
    /// its own slice — it needs an order-INDEPENDENT, AST-gathered name set).
    ///
    /// **Precondition for opting a consumer in:** both the probed map AND `symbols`
    /// must be fully populated before that consumer can run. The three string-array
    /// sites qualify because they are reachable only from expression/lvalue lowering,
    /// which starts after the Nets passes that fill both. **Never opt in a consumer
    /// reachable from `const_eval_in_scope`** — that runs during param folding and
    /// during every generate phase, so it would reproduce the silent generate-body
    /// deletion verbatim. `array_const_vals` (`const_eval.rs`) is exactly such a
    /// consumer: it sits INSIDE `const_eval_in_scope` and must keep the plain walk.
    #[inline]
    pub(crate) fn walk_scopes_key_shadowed(
        &self,
        name: &str,
        hit: impl Fn(&str) -> bool,
    ) -> Option<String> {
        self.walk_scopes_key_inner(name, hit, true)
    }

    fn walk_scopes_key_inner(
        &self,
        name: &str,
        hit: impl Fn(&str) -> bool,
        stop_at_net_binding: bool,
    ) -> Option<String> {
        // GEN-3X-STR (part b): reuse ONE scratch String across the outward walk
        // instead of a fresh `format!` allocation per scope level (byte-identical
        // keys; on a hit the scratch is moved out as the owned return).
        let mut key = String::new();
        // §4.5.589: an owned package routine's text, in a split's window half, binds
        // its declaring package's constant first; a miss walks exactly as before
        // (`decl_scope.rs`).
        if self.decl_probe_key(name, &mut key, &hit) {
            return Some(key);
        }
        self.walk_out_from(
            self.lex.cur_prefix.as_str(),
            name,
            &hit,
            stop_at_net_binding,
            key,
        )
    }

    /// [`Self::walk_scopes_key`]'s outward walk from a SAVED `prefix` (a deferred
    /// reference's lowering scope): the same scope-transparency rule, without the
    /// package-routine probe. ENG-1b: a refused clocking item is found with it from a
    /// function or generate block exactly as `lookup_net_scoped` finds its alias.
    #[inline]
    pub(crate) fn walk_scopes_key_from(
        &self,
        prefix: &crate::ScopeRef,
        name: &str,
        hit: impl Fn(&str) -> bool,
    ) -> Option<String> {
        self.walk_out_from(
            self.lex.names[prefix.0 .0 as usize].as_str(),
            name,
            &hit,
            false,
            String::new(),
        )
    }

    fn walk_out_from(
        &self,
        mut prefix: &str,
        name: &str,
        hit: &impl Fn(&str) -> bool,
        stop_at_net_binding: bool,
        mut key: String,
    ) -> Option<String> {
        use std::fmt::Write;
        loop {
            key.clear();
            if prefix.is_empty() {
                key.push_str(name);
            } else {
                let _ = write!(key, "{prefix}.{name}");
            }
            if hit(&key) {
                return Some(key);
            }
            if stop_at_net_binding && self.symbols.contains_key(&key) {
                return None;
            }
            if prefix.is_empty() {
                return None;
            }
            // The innermost segment about to be stripped: only continue walking
            // outward if it is a generate-block scope (`label[idx]`), an inline-task
            // body-local scope (`$itask$…`), or a frame function/task scope
            // (`$func$…`) — all of which must see the enclosing module's nets (a
            // function/task body may read a module signal; SV §13.4 functions are
            // not pure). A formal/local is registered UNDER the `$func$…`/`$itask$…`
            // segment, so it is still found FIRST (innermost-wins) and correctly
            // shadows a same-named module net; only a NON-local name falls through.
            // Stopping at an instance-boundary segment preserves per-instance name
            // isolation. (A frame body WRITING a module net is still rejected by
            // `validate_frame_body` — the frame-call subset only writes its own
            // locals — so this widens reads without enabling an unsupported write.)
            let last_seg = match prefix.rfind('.') {
                Some(i) => &prefix[i + 1..],
                None => prefix,
            };
            if !Self::is_gen_scope_segment(last_seg)
                && !last_seg.starts_with("$itask$")
                && !last_seg.starts_with("$func$")
                // DUP (round-5): a per-block `$blk$<span>` scope is transparent for
                // outward reads exactly like `$func$`/`$itask$` — a scoped block-local
                // is found FIRST (innermost-wins) while a non-scoped name in the same
                // block still falls through to the enclosing module net.
                && !last_seg.starts_with("$blk$")
            {
                return None;
            }
            prefix = match prefix.rfind('.') {
                Some(i) => &prefix[..i],
                None => "",
            };
        }
    }

    /// The declaring package's key for bare constant `name`, written into `key`,
    /// while the window half runs and the routine does not declare `name` itself.
    /// `walk_scopes_key_inner` asks it before its outward walk.
    fn decl_probe_key(&self, name: &str, key: &mut String, hit: &impl Fn(&str) -> bool) -> bool {
        use std::fmt::Write;
        if self.decl_phase.get() != DeclPhase::Win {
            return false;
        }
        let arm = self.decl_arm.borrow();
        let DeclArm::On(w) = &*arm else {
            return false;
        };
        key.clear();
        let _ = write!(key, "$pkg${}.{name}", w.pkg);
        hit(key) && !w.declares(self, name)
    }

    /// True iff `seg` is a GENERATE-block scope segment (`label[idx]`), as opposed
    /// to an instance-boundary segment (a plain identifier). Generate prefixes
    /// always carry the `[idx]` suffix, so a `[` unambiguously marks them.
    pub(crate) fn is_gen_scope_segment(seg: &str) -> bool {
        seg.contains('[')
    }
}
