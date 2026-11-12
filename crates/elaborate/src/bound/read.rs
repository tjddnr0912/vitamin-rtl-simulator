//! The read lanes over the nine FQ parameter maps (ROADMAP §5.2 BIND-1b, ER §5.1).
//!
//! Two questions several functions spelled alike get one function each:
//!
//! - which maps make a name a hit for the outward scope walk
//!   ([`Elaborator::walk_scopes_key`]'s `hit`): one `hit_*` rule per map set, each
//!   arm a `contains_key`, in the `||` order every one of its sites spelled ;
//! - a name's declared `(width, signed)` and its `real` value by the outward walk over
//!   ONE map: [`Elaborator::name_meta`] (lane META-WALK) and [`Elaborator::name_real`]
//!   (lane REAL-WALK).
//!
//! Structure only: each body is the closure or the `walk_scopes` call its sites
//! spelled, so every answer is PRE's. The arm order matters for instructions, not for
//! the answer (`||` over side-effect-free membership tests) ; a site whose own order
//! differs from its rule's is not re-pointed (it stays at its site, listed below), so
//! no site's code changes order.
//!
//! # The rules (map set ; `||` order ; sites, host fn)
//!
//! | rule | maps, in `||` order | sites |
//! |---|---|---|
//! | `hit_value` | `params` | array_geom.rs `shape_bits` ; edge_gate.rs `binding_encloses` (the value walk) ; ident_route.rs `bare_ident_route` (the guessed key), `ast_reads_guessed_param` |
//! | `hit_value_or_net` | `params`, `symbols` | const_decl_width.rs `narrow_param_decl_width` ; const_wide.rs `narrow_param_bits` ; expr_special.rs `bits_of_view` (the shadow test) ; params.rs `param_sel_range` |
//! | `hit_wide` | `wide_param_bits` | const_eval.rs `wide_param_name_in` ; edge_gate.rs `variable_named_in`, `wide_param_named_in` |
//! | `hit_wide_value_or_net` | `wide_param_bits`, `params`, `symbols` | const_wide.rs `wide_name_bits` ; edge_gate.rs `wide_lsb_range_of` |
//! | `hit_real_value_or_net` | `real_param_val`, `params`, `symbols` | param_query.rs `real_param_is_non_integral` |
//! | `hit_any_const` | `str_param_raw`, `real_param_val`, `params`, `wide_param_bits` | const_bound.rs `unfoldable_reason_in` |
//! | `hit_any_const_or_net` | `str_param_raw`, `wide_param_bits`, `real_param_val`, `params`, `symbols` | ident_route.rs `bare_ident_route` (the per-identifier route) |
//!
//! `symbols` (the net table) is not a guarded map ; it is an arm of the rules that let
//! a net shadow an outer constant (§23.9: the innermost binding wins).
//!
//! # The walks
//!
//! - META-WALK [`Elaborator::name_meta`] = `walk_scopes(n, &param_meta)`: an
//!   independent walk over `param_meta` alone, so a name's value (the `params` walk)
//!   and its type can come from different keys (cell p16: an outer typed
//!   `localparam logic [3:0] N` and an inner `real N` print `A 4 4 20.0`, ROADMAP §2
//!   gen-real-param-shadow). Sites: const_bound.rs `ast_const_leaves_min32` ;
//!   const_eval.rs `const_expr_signed` ; const_fn.rs `const_placement_wide` ;
//!   const_fn_width.rs `const_self_width`, `const_signed_env` ; elab_task.rs
//!   `elab_arg_shape` ; expr_special.rs `bits_of_view` ; ident_route.rs
//!   `bare_ident_route`.
//! - REAL-WALK [`Elaborator::name_real`] = `walk_scopes(n, &real_param_val)`. Sites:
//!   const_real.rs `const_eval_real_in_scope` ; expr_special.rs `bits_of_view` ;
//!   select_scope.rs `signing_operand_admitted` ; strings.rs `expr_mentions_real_opt`.
//!
//! # Left at their sites (said plainly)
//!
//! - A closure whose map set is not a rule's: const_array.rs `const_array_ref_of_base`
//!   (`array_const_vals || params || symbols`, `array_const_vals` is not a guarded map) ;
//!   edge_gate.rs `binding_encloses`'s `other_kind` (`wide || real || str || symbols`,
//!   also asked alone of the value walk's key) ; decl_scope.rs `decl_keep_snapshot`
//!   (six `get`s into `DeclKept`, not a membership test).
//! - A closure whose map set is a rule's in another `||` order (re-pointing it would
//!   change its host function's code ; measured on a build that did, BIND-1b B2):
//!   hier_defer/write.rs `every_name_resolves` (`symbols || params`), const_str.rs
//!   `const_str_at` (`str || real || params || wide || symbols`), edge_gate.rs
//!   `binding_encloses`'s innermost walk (`params || other_kind(k)`).
//! - A closure in its rule's own order whose host function's code moved when it was
//!   re-pointed (the hit became an out-of-line `bool` fn: `tbz w0, #0` became
//!   `cbz w0`, same instruction count) and that no cost cell reaches, so its cost is
//!   UNMEASURED: fold_x.rs `names_real_binding`, param_query.rs
//!   `real_param_lowers_real` (`real_value_or_net`). The sites of
//!   `hit_wide_value_or_net` and `real_param_is_non_integral` moved the same way and
//!   are re-pointed on a counted cost cell (instructions inside +1 %).
//! - `bound_home.rs` W5 lists each of these with its reason.
//! - `lookup_scoped` (the value walk, already one fn), `hier_resolve`'s callers, the
//!   `symbols`-only walks, and the 183 direct reads of the maps in their own combined
//!   orders.
//!
//! # Presence lanes (unchanged)
//!
//! A rule answers "is the name bound by one of these maps at this key", nothing about
//! what the entry means: VALUE-ABSENT, META-ABSENT, RANGE-ABSENT, STR-KEEPS-VALUE and
//! the rest stay as `bound/mod.rs` tables them.

use crate::Elaborator;

impl Elaborator<'_> {
    /// META-WALK: the declared `(width, signed)` of `n` by the outward walk over
    /// `param_meta` alone (it may answer from another key than the value walk).
    #[inline]
    pub(crate) fn name_meta(&self, n: &str) -> Option<(u32, bool)> {
        self.walk_scopes(n, &self.param_meta)
    }

    /// REAL-WALK: the `real` value of `n` by the outward walk over `real_param_val`.
    #[inline]
    pub(crate) fn name_real(&self, n: &str) -> Option<f64> {
        self.walk_scopes(n, &self.real_param_val)
    }

    /// `params`.
    #[inline]
    pub(crate) fn hit_value(&self, k: &str) -> bool {
        self.params.contains_key(k)
    }

    /// `params || symbols`.
    #[inline]
    pub(crate) fn hit_value_or_net(&self, k: &str) -> bool {
        self.params.contains_key(k) || self.symbols.contains_key(k)
    }

    /// `wide_param_bits`.
    #[inline]
    pub(crate) fn hit_wide(&self, k: &str) -> bool {
        self.wide_param_bits.contains_key(k)
    }

    /// `wide_param_bits || params || symbols`.
    #[inline]
    pub(crate) fn hit_wide_value_or_net(&self, k: &str) -> bool {
        self.wide_param_bits.contains_key(k)
            || self.params.contains_key(k)
            || self.symbols.contains_key(k)
    }

    /// `real_param_val || params || symbols`.
    #[inline]
    pub(crate) fn hit_real_value_or_net(&self, k: &str) -> bool {
        self.real_param_val.contains_key(k)
            || self.params.contains_key(k)
            || self.symbols.contains_key(k)
    }

    /// `str_param_raw || real_param_val || params || wide_param_bits`.
    #[inline]
    pub(crate) fn hit_any_const(&self, k: &str) -> bool {
        self.str_param_raw.contains_key(k)
            || self.real_param_val.contains_key(k)
            || self.params.contains_key(k)
            || self.wide_param_bits.contains_key(k)
    }

    /// `str_param_raw || wide_param_bits || real_param_val || params || symbols`.
    #[inline]
    pub(crate) fn hit_any_const_or_net(&self, k: &str) -> bool {
        self.str_param_raw.contains_key(k)
            || self.wide_param_bits.contains_key(k)
            || self.real_param_val.contains_key(k)
            || self.params.contains_key(k)
            || self.symbols.contains_key(k)
    }
}
