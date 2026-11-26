//! The read lanes over the nine FQ parameter maps (ROADMAP §5.2 BIND-1b, ER §5.1).
//!
//! Two questions several functions spelled alike get one function each:
//!
//! - which maps make a name a hit for the outward scope walk, for a map set that names
//!   one of the nine maps: one `hit_*` lane per set, each arm a `contains_at`, in the `||`
//!   order every one of its sites spelled. Since §5.2 Q-DECLKEY DK2 a walk site calls its
//!   set's named entry (`lexscope/walk.rs`, `walk_<kind>_<set>`, one call of a private walk
//!   with a closure literal), whose closure calls the lane ; a set over other stores only
//!   (`symbols`, `pkg_var_aliases`, ..) is an inline closure of its entry ;
//! - a name's declared `(width, signed)` and its `real` value, each by the outward walk
//!   over ONE map: [`Elaborator::name_meta`] (lane META-WALK) and
//!   [`Elaborator::name_real`] (lane REAL-WALK).
//!
//! Structure only: each lane's body is the closure its sites spelled, so every answer is
//! PRE's. The arm order matters for instructions, not for the answer (`||` over
//! side-effect-free membership tests) ; one map set spelled in two orders is two lanes, not
//! merged, so no site's probes change order.
//!
//! # The lanes (map set ; `||` order ; named entry ; sites, host fn)
//!
//! | lane | maps, in `||` order | entry | sites |
//! |---|---|---|---|
//! | `hit_value` | `params` | `walk_key_value` | array_geom.rs `shape_bits` ; edge_gate.rs `binding_encloses` (the value walk) ; ident_route.rs `bare_ident_route` (the guessed key), `ast_reads_guessed_param` ; scope.rs `lookup_scoped` (the value walk) |
//! | `hit_value_or_net` | `params`, `symbols` | `walk_key_value_or_net` | const_decl_width.rs `narrow_param_decl_width` ; const_wide.rs `narrow_param_bits` ; expr_special.rs `bits_of_view` (the shadow test) ; params.rs `param_sel_range` |
//! | `hit_wide` | `wide_param_bits` | `walk_key_wide` | const_eval.rs `wide_param_name_in` ; edge_gate.rs `variable_named_in`, `wide_param_named_in` |
//! | `hit_wide_value_or_net` | `wide_param_bits`, `params`, `symbols` | `walk_key_wide_value_or_net` | const_wide.rs `wide_name_bits` ; edge_gate.rs `wide_lsb_range_of` |
//! | `hit_real_value_or_net` | `real_param_val`, `params`, `symbols` | `walk_key_real_value_or_net` | param_query.rs `real_param_is_non_integral`, `real_param_lowers_real` ; const_fn_width/fold_x.rs `names_real_binding` |
//! | `hit_any_const` | `str_param_raw`, `real_param_val`, `params`, `wide_param_bits` | `walk_key_any_const` | const_bound.rs `unfoldable_reason_in` |
//! | `hit_any_const_or_net` | `str_param_raw`, `wide_param_bits`, `real_param_val`, `params`, `symbols` | `walk_key_any_const_or_net` | ident_route.rs `bare_ident_route` (the per-identifier route) |
//! | `hit_meta` | `param_meta` | `walk_key_meta` | `name_meta` (META-WALK) |
//! | `hit_real` | `real_param_val` | `walk_key_real` | `name_real` (REAL-WALK) |
//! | `hit_array_const_value_or_net` | `array_const_vals`, `params`, `symbols` | `walk_key_array_const_value_or_net` | const_array.rs `const_array_ref_of_base` |
//! | `hit_str_real_value_wide_or_net` | `str_param_raw`, `real_param_val`, `params`, `wide_param_bits`, `symbols` | `walk_key_str_real_value_wide_or_net` | const_str.rs `const_str_at` |
//! | `hit_value_or_other_kind` | `params`, `wide_param_bits`, `real_param_val`, `str_param_raw`, `symbols` | `walk_key_value_or_other_kind` | edge_gate.rs `binding_encloses` (the innermost walk) |
//! | `hit_net_or_value` | `symbols`, `params` | `walk_key_net_or_value` | hier_defer/write.rs `every_name_resolves` |
//!
//! `symbols` (the net table) and `array_const_vals` are not guarded maps ; `symbols` is an
//! arm of the lanes that let a net shadow an outer constant (§23.9: the innermost binding
//! wins).
//!
//! # The walks
//!
//! - META-WALK [`Elaborator::name_meta`] = `walk_key_meta`, then `param_meta`'s entry at
//!   the found key: an independent walk over `param_meta` alone, so a name's
//!   value (the `params` walk) and its type can come from different keys (cell p16: an
//!   outer typed `localparam logic [3:0] N` and an inner `real N` print `A 4 4 20.0`,
//!   ROADMAP §2 gen-real-param-shadow). Sites: const_bound.rs
//!   `ast_const_leaves_min32` ; const_eval.rs `const_expr_signed` ; const_fn.rs
//!   `const_placement_wide` ; const_fn_width.rs `const_self_width`, `const_signed_env` ;
//!   elab_task.rs `elab_arg_shape` ; expr_special.rs `bits_of_view` ; ident_route.rs
//!   `bare_ident_route`.
//! - REAL-WALK [`Elaborator::name_real`] = `walk_key_real`, then `real_param_val`'s
//!   entry at the found key. Sites: const_real.rs
//!   `const_eval_real_in_scope` ; expr_special.rs `bits_of_view` ; select_scope.rs
//!   `signing_operand_admitted` ; strings.rs `expr_mentions_real_opt`.
//!
//! # What DK2 moved here, and what is not measured (said plainly)
//!
//! - Four of the six multi-map walk closures left at their sites before DK2 (no rule's set, or a
//!   rule's in another order: const_array.rs, const_str.rs, edge_gate.rs's innermost walk,
//!   hier_defer/write.rs ; BIND-1b B2: W/bind1/build_b/asm/B2_SUMMARY.txt `cmp_post_extra`) ;
//!   each is a lane above, in its own order. edge_gate.rs `binding_encloses`'s `other_kind`
//!   (the found key's four maps) and decl_scope.rs `decl_keep_snapshot` (six `get_new`s
//!   into `DeclKept`, not a membership test) stay at their sites.
//! - fold_x.rs `names_real_binding` and param_query.rs `real_param_lowers_real` spelled
//!   `hit_real_value_or_net`'s own text, left by R7 (re-pointing moved the host fn's code:
//!   `tbz w0, #0` became `cbz w0`, and no cost cell reaches them) ; DK2 re-points them by
//!   construction (every site asks one walk) and their cost is still UNMEASURED.
//! - Re-pointed at BIND-1b, evidence = the disassembly (BIND-1 rulings R16, R17): the
//!   host fns of `real_param_is_non_integral`, `wide_name_bits` and `wide_lsb_range_of`
//!   have the same instruction count, with the one `tbz w0, #0` -> `cbz w0` encoding
//!   above (W/bind1/build_b/asm/). The cost cells counted beside it (instructions, PRE vs
//!   POST, W/bind1/build_b/ab.SUMMARY.txt): g_sgn -0.11 % (`wide_lsb_range_of`),
//!   k_fnret +0.00 % (`real_param_is_non_integral`, one use: it measures nothing per
//!   use), y_bind_gen +0.13 %, y_import_inst -0.08 %, y_red_wide -0.00 %
//!   (`wide_name_bits`).
//! - `bound_home.rs` W5 lists each exception with its reason ; W5d counts each lane's calls
//!   per (file, fn) (since DK2 the lanes' calls are the named entries') ; and no token of
//!   the nine maps is in `lexscope/walk.rs`, so no entry's closure reads one inline.
//! - `hier_resolve`'s callers and the direct reads of the maps in their own combined
//!   orders stay where they are: 81 lines at f17a0ff6 by a line census
//!   (`git grep -P '\.<one of the nine>\b(?!\s*\()'` over crates/elaborate/src outside
//!   `bound/`, `//` lines out ; it also counts other structs' `.params` and cannot see a
//!   read through a local or a helper fn).
//!
//! # Presence lanes (unchanged)
//!
//! A rule answers "is the name bound by one of these maps at this key", nothing about
//! what the entry means: VALUE-ABSENT, META-ABSENT, RANGE-ABSENT, STR-KEEPS-VALUE and
//! the rest stay as `bound/mod.rs` tables them.

use crate::{AtKey, Elaborator};

impl Elaborator<'_> {
    /// META-WALK: the declared `(width, signed)` of `n` by the outward walk over
    /// `param_meta` alone (it may answer from another key than the value walk).
    #[inline]
    pub(crate) fn name_meta(&self, n: &str) -> Option<(u32, bool)> {
        self.walk_key_meta(n)
            .and_then(|k| self.param_meta.get_found(&k).copied())
    }

    /// REAL-WALK: the `real` value of `n` by the outward walk over `real_param_val`.
    #[inline]
    pub(crate) fn name_real(&self, n: &str) -> Option<f64> {
        self.walk_key_real(n)
            .and_then(|k| self.real_param_val.get_found(&k).copied())
    }

    /// `params`.
    #[inline]
    pub(crate) fn hit_value(&self, k: &AtKey) -> bool {
        self.params.contains_at(k)
    }

    /// `params || symbols`.
    #[inline]
    pub(crate) fn hit_value_or_net(&self, k: &AtKey) -> bool {
        self.params.contains_at(k) || self.symbols.contains_at(k)
    }

    /// `wide_param_bits`.
    #[inline]
    pub(crate) fn hit_wide(&self, k: &AtKey) -> bool {
        self.wide_param_bits.contains_at(k)
    }

    /// `wide_param_bits || params || symbols`.
    #[inline]
    pub(crate) fn hit_wide_value_or_net(&self, k: &AtKey) -> bool {
        self.wide_param_bits.contains_at(k)
            || self.params.contains_at(k)
            || self.symbols.contains_at(k)
    }

    /// `real_param_val || params || symbols`.
    #[inline]
    pub(crate) fn hit_real_value_or_net(&self, k: &AtKey) -> bool {
        self.real_param_val.contains_at(k)
            || self.params.contains_at(k)
            || self.symbols.contains_at(k)
    }

    /// `str_param_raw || real_param_val || params || wide_param_bits`.
    #[inline]
    pub(crate) fn hit_any_const(&self, k: &AtKey) -> bool {
        self.str_param_raw.contains_at(k)
            || self.real_param_val.contains_at(k)
            || self.params.contains_at(k)
            || self.wide_param_bits.contains_at(k)
    }

    /// `str_param_raw || wide_param_bits || real_param_val || params || symbols`.
    #[inline]
    pub(crate) fn hit_any_const_or_net(&self, k: &AtKey) -> bool {
        self.str_param_raw.contains_at(k)
            || self.wide_param_bits.contains_at(k)
            || self.real_param_val.contains_at(k)
            || self.params.contains_at(k)
            || self.symbols.contains_at(k)
    }

    /// `param_meta` (§5.2 Q-DECLKEY DK2: `walk_key_meta`, META-WALK's hit).
    #[inline]
    pub(crate) fn hit_meta(&self, k: &AtKey) -> bool {
        self.param_meta.contains_at(k)
    }

    /// `real_param_val` (DK2: `walk_key_real`, REAL-WALK's hit).
    #[inline]
    pub(crate) fn hit_real(&self, k: &AtKey) -> bool {
        self.real_param_val.contains_at(k)
    }

    /// `array_const_vals || params || symbols` (DK2: `walk_key_array_const_value_or_net`).
    #[inline]
    pub(crate) fn hit_array_const_value_or_net(&self, k: &AtKey) -> bool {
        self.array_const_vals.contains_at(k)
            || self.params.contains_at(k)
            || self.symbols.contains_at(k)
    }

    /// `str_param_raw || real_param_val || params || wide_param_bits || symbols` (DK2:
    /// `walk_key_str_real_value_wide_or_net`, `hit_any_const_or_net`'s set in its site's own
    /// order).
    #[inline]
    pub(crate) fn hit_str_real_value_wide_or_net(&self, k: &AtKey) -> bool {
        self.str_param_raw.contains_at(k)
            || self.real_param_val.contains_at(k)
            || self.params.contains_at(k)
            || self.wide_param_bits.contains_at(k)
            || self.symbols.contains_at(k)
    }

    /// `params || wide_param_bits || real_param_val || str_param_raw || symbols` (DK2:
    /// `walk_key_value_or_other_kind`, `hit_any_const_or_net`'s set in its site's own order).
    #[inline]
    pub(crate) fn hit_value_or_other_kind(&self, k: &AtKey) -> bool {
        self.params.contains_at(k)
            || self.wide_param_bits.contains_at(k)
            || self.real_param_val.contains_at(k)
            || self.str_param_raw.contains_at(k)
            || self.symbols.contains_at(k)
    }

    /// `symbols || params` (DK2: `walk_key_net_or_value`, `hit_value_or_net`'s set in its site's
    /// own order).
    #[inline]
    pub(crate) fn hit_net_or_value(&self, k: &AtKey) -> bool {
        self.symbols.contains_at(k) || self.params.contains_at(k)
    }
}
