//! value -- the home of elaborate's constant evaluator (ROADMAP §5.2 VAL-1..3, owner 2026-11-09).
//! LANES (the five cores ; each is an entry by its own name, in its own file):
//!   IN   const_eval_in_scope   scope: live (module / instance)       context: none (unlimited i64)        no: None
//!   ENV  eval_const_env        scope: frame (env, envw, depth)       context: none                         no: None
//!   CTX  fold_ctx (+ fold_self, fold_assign, fold_shift_count)  frame  context: (width, signed) <= 64       no: No | Four{v, x, why}
//!   REG  fold_region (+ fold_bits_at / _at0, fold_self_bits)   a name callback, no self   context: width   no: None ; a value is 4-state
//!   HIER env_fold              scope: FoldScope (live | child env)   context: none ; i32 result check       no: None ; Some((v, wide))
//! The cores call each other: IN -> CTX (const_int_selfdet) -> ENV (CTX's leaf arm) -> IN (an empty frame, call-free past depth 0).
//! `arms`   the arms two or more cores spelled (VAL-1).
//! `chains` a consumer's sequence of lanes, once: `param_decl_fold` (module body, package, generate scope, and the header after its
//!          override channels ; the header asks step 1, the untyped fill, only when the default binds).
//! `lanes`  a core asked in a named scope: `fold_self_wide` (REG, live scope ; the eleven adapters' first fold).
//!          Release build (VAL-2 D5): `fold_self_wide` has no symbol (inlined, each adapter's size unchanged) ; `param_decl_fold`
//!          is one out-of-line fn, one call per declaration in each binder.
//!          A row that writes one arm for cores of different result types adds `fold_in(lane, e) -> Folded` here ; the record
//!          `Folded` (value/folded.rs) lands with its first producer (VAL-3).
//! Chains that stay at their consumer (PRE's order, named: docs/roadmap/5.2/VAL-2.md): param_init_x (CTX then IN), the range router
//!   (VAL-3), const_bound_u32 / _signed, the override channels (eager), `$clog2`, the generate condition, the delay readers, decl_split.
//! "No" is not one thing: None is declined AND refused ; these readers take a decline as a value (their owners in VAL-2.md): const_fn.rs
//!   bind_const_decl and the formal binder (width 0), inline_fold.rs dims, const_decl_wsign's `.ok()?` x2, const_fn_ret_wsign,
//!   param_decl_width_opt, eval_param_init's `.ok().flatten()`.
//! `had_error` is read 6 times, each "any error so far" ; no fold entry can set it (`&self`).
//! ENFORCED by test (elaborate/tests/value_entry_home.rs, token shapes per (file, fn), `#[cfg(test)]` skipped): E1 one `fn` each of the
//!   entry list ; E2 the call sites of the non-IN cores and frame entries ; E3 the binder chain's five members, in order, here only,
//!   steps 2-5 each inside an `or_else ( | |` and step 1 inside `fill . then ( | |` (lazy ; `then_some (` is refused) ; E4
//!   `wide_name_bits` passed to `fold_self_bits` here only ; E5 the `had_error` readers ; E6 `unwrap_or ( ( 0 , false ) )`.
//! REVIEW-ONLY (no test sees them): an entry reached through a fn value or a macro ; a chain spelled with `match` / `if let` instead of
//!   `or_else` ; a decline read after a `let` ; the predicate readers (`.is_some()` over an entry, 23 sites) ; every chain not listed ;
//!   a live-scope REG ask spelled through `fold_bits_at(e, 0, ..)` or a `param_leaf_bits` closure (E4 does not see it ; E2's
//!   fold_bits_at / fold_self_bits counts do) ; a class parameter's default (bound by the parser's specialization, not by a binder here).

pub(crate) mod arms;
pub(crate) mod chains;
pub(crate) mod lanes;
