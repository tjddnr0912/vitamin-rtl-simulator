//! The outward scope walk (§5.2 Q-DECLKEY DK1-4a): the key-returning walk of a name from the current
//! scope (or a carried one) outward through the transparent generate / `$func$` / `$itask$` / `$blk$`
//! scopes, its saved-prefix forms, the package window's probe it asks first, and the generate-segment
//! test it stops on — moved here whole from `scope.rs`, `generate.rs` and `decl_scope.rs`. They are
//! Elaborator methods, tier E at DK1-4a and private to this file since DK2 (the segment test:
//! `pub(super)` ; THE HIT SETS below) ; inside, they read the current scope's text and a carried
//! scope's text as this module's own fields, not through the bridges. Since DK1-4b they are typed:
//! `hit` is handed each candidate as an [`AtKey`] (the walk's one scratch text, re-written per
//! scope level ; DK1-4b-1) and the walk returns the candidate it accepted as a [`DeclKey`], the
//! scratch text moved into it, so a hit allocates nothing the text walk did not (DK1-4b-2).
//!
//! THE HIT SETS (§5.2 Q-DECLKEY DK2): the four generic walks are private to this file ; code outside
//! asks one of the named entries at its foot, one per hit set, `walk_<kind>_<set>` (kind `key`,
//! `shadowed`, `from`, `above`: the walk it calls). Each entry's body is ONE call of its walk with a
//! closure literal, the closure its sites spelled before DK2 (the same probes in the same `||` order:
//! an inline `contains_at` over a store that is none of the nine FQ parameter maps ; a `hit_*` lane of
//! `bound/read.rs` for a set that names one ; `held.rs`'s `hit_clk_refused` for the refused clocking
//! items), so no fn outside this file takes a predicate over declaration keys and nothing selects a
//! set at run time. The generic value walk over one caller's store (`scope.rs`) is deleted: its three
//! callers ask an entry and read the store's entry at the found key themselves (`lookup_scoped`,
//! `name_meta`, `name_real`).

use super::{AtKey, DeclKey};
use crate::decl_scope::{DeclArm, DeclPhase};
use crate::Elaborator;

impl Elaborator<'_> {
    /// The innermost key `<scope>.<name>` whose candidate satisfies `hit`, searching the current
    /// scope and every scope enclosing it while the stripped segment is transparent — ONE source of
    /// truth for the visibility rule, so key-level consumers (the modport read-only check, the
    /// iface-instance lookup) can never drift from the value-level lookups.
    #[inline(always)]
    fn walk_scopes_key(&self, name: &str, hit: impl Fn(&AtKey) -> bool) -> Option<DeclKey> {
        self.walk_scopes_key_inner(name, hit, false)
    }

    /// [`Self::walk_scopes_key`]'s shape for a SAVED prefix: the innermost key
    /// `<scope>.<name>` that satisfies `hit`, searching `prefix` and every scope
    /// enclosing it.
    ///
    /// The deferred hierarchical resolvers run after `cur_prefix` has moved on, so they
    /// carry the prefix that was live when the reference was LOWERED and cannot use the
    /// ambient walk. Diagnostic-only today (V33-3's enum-label receiver test).
    #[inline(always)]
    fn scoped_key_at_or_above(
        &self,
        prefix: &crate::ScopeRef,
        name: &str,
        hit: impl Fn(&AtKey) -> bool,
    ) -> Option<DeclKey> {
        let prefix = self.lex.names[prefix.0 .0 as usize].as_str();
        let mut scope = prefix;
        loop {
            let key = AtKey(if scope.is_empty() {
                name.to_string()
            } else {
                format!("{scope}.{name}")
            });
            if hit(&key) {
                return Some(DeclKey(key.0));
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
    #[inline(always)]
    fn walk_scopes_key_shadowed(
        &self,
        name: &str,
        hit: impl Fn(&AtKey) -> bool,
    ) -> Option<DeclKey> {
        self.walk_scopes_key_inner(name, hit, true)
    }

    #[inline(always)]
    fn walk_scopes_key_inner(
        &self,
        name: &str,
        hit: impl Fn(&AtKey) -> bool,
        stop_at_net_binding: bool,
    ) -> Option<DeclKey> {
        // GEN-3X-STR (part b): reuse ONE scratch String across the outward walk
        // instead of a fresh `format!` allocation per scope level (byte-identical
        // keys; on a hit the scratch is moved out as the owned return).
        let mut key = AtKey(String::new());
        // §4.5.589: an owned package routine's text, in a split's window half, binds
        // its declaring package's constant first; a miss walks exactly as before
        // (`decl_scope.rs`).
        if self.decl_probe_key(name, &mut key, &hit) {
            return Some(DeclKey(key.0));
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
    #[inline(always)]
    fn walk_scopes_key_from(
        &self,
        prefix: &crate::ScopeRef,
        name: &str,
        hit: impl Fn(&AtKey) -> bool,
    ) -> Option<DeclKey> {
        self.walk_out_from(
            self.lex.names[prefix.0 .0 as usize].as_str(),
            name,
            &hit,
            false,
            AtKey(String::new()),
        )
    }

    #[inline(always)]
    fn walk_out_from(
        &self,
        mut prefix: &str,
        name: &str,
        hit: &impl Fn(&AtKey) -> bool,
        stop_at_net_binding: bool,
        mut key: AtKey,
    ) -> Option<DeclKey> {
        use std::fmt::Write;
        loop {
            key.0.clear();
            if prefix.is_empty() {
                key.0.push_str(name);
            } else {
                let _ = write!(key.0, "{prefix}.{name}");
            }
            if hit(&key) {
                return Some(DeclKey(key.0));
            }
            if stop_at_net_binding && self.symbols.contains_at(&key) {
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
    #[inline(always)]
    fn decl_probe_key(&self, name: &str, key: &mut AtKey, hit: &impl Fn(&AtKey) -> bool) -> bool {
        use std::fmt::Write;
        if self.decl_phase.get() != DeclPhase::Win {
            return false;
        }
        let arm = self.decl_arm.borrow();
        let DeclArm::On(w) = &*arm else {
            return false;
        };
        key.0.clear();
        let _ = write!(key.0, "$pkg${}.{name}", w.pkg);
        hit(key) && !w.declares(self, name)
    }

    // ── the named entries (§5.2 Q-DECLKEY DK2): one per hit set, each ONE call of its walk ──

    /// [`Self::walk_scopes_key`] over the hit set `params` (`hit_value`).
    #[inline]
    pub(crate) fn walk_key_value(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.hit_value(k),
        )
    }

    /// [`Self::walk_scopes_key`] over the hit set `params || symbols` (`hit_value_or_net`).
    #[inline]
    pub(crate) fn walk_key_value_or_net(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.hit_value_or_net(k),
        )
    }

    /// [`Self::walk_scopes_key`] over the hit set `wide_param_bits` (`hit_wide`).
    #[inline]
    pub(crate) fn walk_key_wide(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.hit_wide(k),
        )
    }

    /// [`Self::walk_scopes_key`] over the hit set `wide_param_bits || params || symbols`
    /// (`hit_wide_value_or_net`).
    #[inline]
    pub(crate) fn walk_key_wide_value_or_net(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.hit_wide_value_or_net(k),
        )
    }

    /// [`Self::walk_scopes_key`] over the hit set `real_param_val || params || symbols`
    /// (`hit_real_value_or_net`).
    #[inline]
    pub(crate) fn walk_key_real_value_or_net(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.hit_real_value_or_net(k),
        )
    }

    /// [`Self::walk_scopes_key`] over the hit set `str_param_raw || real_param_val || params ||
    /// wide_param_bits` (`hit_any_const`).
    #[inline]
    pub(crate) fn walk_key_any_const(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.hit_any_const(k),
        )
    }

    /// [`Self::walk_scopes_key`] over the hit set `str_param_raw || wide_param_bits ||
    /// real_param_val || params || symbols` (`hit_any_const_or_net`).
    #[inline]
    pub(crate) fn walk_key_any_const_or_net(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.hit_any_const_or_net(k),
        )
    }

    /// [`Self::walk_scopes_key`] over the hit set `param_meta` (`hit_meta`: META-WALK,
    /// `name_meta`).
    #[inline]
    pub(crate) fn walk_key_meta(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.hit_meta(k),
        )
    }

    /// [`Self::walk_scopes_key`] over the hit set `real_param_val` (`hit_real`: REAL-WALK,
    /// `name_real`).
    #[inline]
    pub(crate) fn walk_key_real(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.hit_real(k),
        )
    }

    /// [`Self::walk_scopes_key`] over the hit set `array_const_vals || params || symbols`
    /// (`hit_array_const_value_or_net`).
    #[inline]
    pub(crate) fn walk_key_array_const_value_or_net(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.hit_array_const_value_or_net(k),
        )
    }

    /// [`Self::walk_scopes_key`] over the hit set `str_param_raw || real_param_val || params ||
    /// wide_param_bits || symbols` (`hit_str_real_value_wide_or_net`).
    #[inline]
    pub(crate) fn walk_key_str_real_value_wide_or_net(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.hit_str_real_value_wide_or_net(k),
        )
    }

    /// [`Self::walk_scopes_key`] over the hit set `params || wide_param_bits || real_param_val ||
    /// str_param_raw || symbols` (`hit_value_or_other_kind`).
    #[inline]
    pub(crate) fn walk_key_value_or_other_kind(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.hit_value_or_other_kind(k),
        )
    }

    /// [`Self::walk_scopes_key`] over the hit set `symbols || params` (`hit_net_or_value`).
    #[inline]
    pub(crate) fn walk_key_net_or_value(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.hit_net_or_value(k),
        )
    }

    /// [`Self::walk_scopes_key`] over the hit set `symbols`.
    #[inline]
    pub(crate) fn walk_key_net(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.symbols.contains_at(k),
        )
    }

    /// [`Self::walk_scopes_key`] over the hit set `pkg_var_aliases`.
    #[inline]
    pub(crate) fn walk_key_pkg_var_alias(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.pkg_var_aliases.contains_at(k),
        )
    }

    /// [`Self::walk_scopes_key`] over the hit set `iface_insts`.
    #[inline]
    pub(crate) fn walk_key_iface_inst(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.iface_insts.contains_at(k),
        )
    }

    /// [`Self::walk_scopes_key`] over the hit set `genvar_decls`.
    #[inline]
    pub(crate) fn walk_key_genvar(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key(
            name,
            #[inline(always)]
            |k| self.genvar_decls.contains_at(k),
        )
    }

    /// [`Self::walk_scopes_key_shadowed`] over the hit set `string_array_elems`.
    #[inline]
    pub(crate) fn walk_shadowed_string_array_elem(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key_shadowed(
            name,
            #[inline(always)]
            |k| self.string_array_elems.contains_at(k),
        )
    }

    /// [`Self::walk_scopes_key_shadowed`] over the hit set `fixed_string_dyn_key`.
    #[inline]
    pub(crate) fn walk_shadowed_fixed_string_dyn(&self, name: &str) -> Option<DeclKey> {
        self.walk_scopes_key_shadowed(
            name,
            #[inline(always)]
            |k| self.fixed_string_dyn_key.contains_at(k),
        )
    }

    /// [`Self::walk_scopes_key_from`] over the hit set `symbols ||` a refused clocking item's alias
    /// (`held.rs` `hit_clk_refused`).
    #[inline]
    pub(crate) fn walk_from_net_or_refused_clk(
        &self,
        prefix: &crate::ScopeRef,
        name: &str,
    ) -> Option<DeclKey> {
        self.walk_scopes_key_from(
            prefix,
            name,
            #[inline(always)]
            |k| self.symbols.contains_at(k) || self.hit_clk_refused(k),
        )
    }

    /// [`Self::scoped_key_at_or_above`] over the hit set `gen_singleton_labels`.
    #[inline]
    pub(crate) fn walk_above_gen_singleton_label(
        &self,
        prefix: &crate::ScopeRef,
        name: &str,
    ) -> Option<DeclKey> {
        self.scoped_key_at_or_above(
            prefix,
            name,
            #[inline(always)]
            |k| self.gen_singleton_labels.contains_at(k),
        )
    }

    /// [`Self::scoped_key_at_or_above`] over the hit set `gen_loop_labels`.
    #[inline]
    pub(crate) fn walk_above_gen_loop_label(
        &self,
        prefix: &crate::ScopeRef,
        name: &str,
    ) -> Option<DeclKey> {
        self.scoped_key_at_or_above(
            prefix,
            name,
            #[inline(always)]
            |k| self.gen_loop_labels.contains_at(k),
        )
    }

    /// [`Self::scoped_key_at_or_above`] over the hit set `enum_label_types`.
    #[inline]
    pub(crate) fn walk_above_enum_label_type(
        &self,
        prefix: &crate::ScopeRef,
        name: &str,
    ) -> Option<DeclKey> {
        self.scoped_key_at_or_above(
            prefix,
            name,
            #[inline(always)]
            |k| self.enum_label_types.contains_at(k),
        )
    }

    /// True iff `seg` is a GENERATE-block scope segment (`label[idx]`), as opposed
    /// to an instance-boundary segment (a plain identifier). Generate prefixes
    /// always carry the `[idx]` suffix, so a `[` unambiguously marks them.
    pub(super) fn is_gen_scope_segment(seg: &str) -> bool {
        seg.contains('[')
    }
}
