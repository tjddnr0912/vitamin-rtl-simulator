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
//!          override channels ; the header asks step 1, the untyped fill, only when the default binds) ; `range_bounds_checked` (VAL-3:
//!          a declared range's two bounds folded by the range router, then checked, msb first -- the four loud declaration sites).
//! `lanes`  a core asked in a named scope: `fold_self_wide` (REG, live scope ; the eleven adapters' first fold).
//!          Release build (VAL-2 D5): `fold_self_wide` has no symbol (inlined, each adapter's size unchanged) ; `param_decl_fold`
//!          is one out-of-line fn, one call per declaration in each binder.  (VAL-3 B4) `range_bounds_checked` has no symbol
//!          (inlined at its four sites ; packed_extents 468, check_param_decl_range 156, array_dim_extents 684 bytes unchanged,
//!          range_to_dims_opt 908 -> 912).
//!          A row that writes one arm for cores of different result types adds `fold_in(lane, e) -> Folded` here ; the record
//!          `Folded` (value/folded.rs) lands with its first producer (not VAL-3, which builds no record: the first arm written
//!          across cores of different result types).
//! Chains that stay at their consumer (PRE's order, named: docs/roadmap/5.2/VAL-2.md): param_init_x (CTX then IN), the range router
//!   (below), const_bound_u32 / _signed, the override channels (eager), `$clog2`, the generate condition, the delay readers, decl_split.
//! THE RANGE ROUTER (VAL-3 ; one body, not edited: const_select.rs `const_range_bound_fold` -> `decl_split(|| _at(e))` ->
//!   const_fn_width/fold_x.rs `range_bound_fold_ev(e, want_ev)`).  Paths: A the bound holds a parameter select -> `const_int_selfdet` only ;
//!   B `fold_self` (CTX, self-determined) answers Val(v) with v <= i32::MAX (`hdl_ast::range_rules::bound_step`) -> v ; C otherwise IN
//!   (`const_eval_in_scope`) -> its value, which may differ from the first fold's (g_negu8 4294967295 vs -1, x04, g_w32u) ; D `decl_split`
//!   (a package routine's text: PRE's half, then the declaring scope's) ; E `check_const_range_bound` re-runs the router with the evidence
//!   (`bound_x_reason`, only on a folded bound holding `/ % **`).  What each step drops: c1 the literal leaf (x / z plane, > 64 bits ->
//!   None ; a 64-bit literal read `as i64`) ; c2 fold_ctx's mask at the context width (64: the pattern itself) ; c3 fold_self's (w, sg) ;
//!   c4 A's `legacy()` (Four / No -> None) ; c5 bound_step keeps only Val(v <= i32::MAX) ; c6 C's IN value replaces the first fold ;
//!   c7 the evidence (want_ev false) ; c8 decl_split keeps one half ; c9 each caller's narrowing (unwrap_or(0), clamp, a Lane spelling).
//!   WRAP LANE: the router's Option<i64> is PRE's reading -- a 64-bit unsigned bound >= 2^63 reads negative (r11 `A bw=2 l=-1`), path C
//!   returns IN's value, not the self-determined one -- and every present consumer reads it (the four chain sites, RangeFacts::lazy /
//!   ::pair's msb / lsb, the direct callers).  The first reader of anything more (the typed bound: value, width, sign of the first fold)
//!   is Q-VALRULES (b) R6.1, the R1v row's first commit (VAL-3.md: `BoundFold { v, first, w, sg }` beside CFold, elaborate-private ;
//!   a name leaf's type is not true until BIND-1 / S1v-V: p5_uns_kw_s).  Decline readers fed by the router through range_facts: R7
//!   (inline_fn.rs const_fn_ret_wsign, K12 ; owner R6.1) and R8 (params.rs param_decl_width_opt, K13 ; owner BIND-1).
//!   THE CHECK (`range_bounds_checked`, chains.rs): what it orders and what it does not --
//!   (1) the FIRST checker of a declaration speaks: a module net's range reaches packed_extents (S1, netdecl.rs) before
//!       range_to_dims_opt (S4) ; a port (ports.rs) and a function return (func_return_dims_opt) reach S4 first ;
//!   (2) the checks dedup by the bound's span (`reported_bad_bounds`, `reported_x_bounds`), so a later checker of the same bound and a
//!       second instance of the same declaration print nothing ; diagnostics come in CALL order, not span order ;
//!   (3) parser-synthesized ranges give msb and lsb ONE span (assertions.rs synth_bit_range, range_rules.rs md_flat_range) and
//!       expr_primary.rs dec_range gives every synthesized struct vector Span(0,0): the dedup collapses them, as in PRE ;
//!   (4) a check added inside range_bounds_checked reaches all four sites, S3's unpacked `[m:l]` (array_dim_extents) included ;
//!   (5) `Dim::Size` (array_geom.rs, `[N]`: IN + check_const_range_bound) is the one declared-bound check outside the chain.
//! "No" is not one thing: None is declined AND refused ; these readers take a decline as a value (their owners in VAL-2.md): const_fn.rs
//!   bind_const_decl and the formal binder (width 0), inline_fold.rs dims, const_decl_wsign's `.ok()?` x2, const_fn_ret_wsign,
//!   param_decl_width_opt, eval_param_init's `.ok().flatten()`.
//! `had_error` is read 6 times, each "any error so far" ; no fold entry can set it (`&self`).
//! ENFORCED by test (elaborate/tests/value_entry_home.rs, token shapes per (file, fn), `#[cfg(test)]` skipped): E1 one `fn` each of the
//!   entry list ; E2 the call sites of the non-IN cores and frame entries ; E3 the binder chain's five members, in order, here only,
//!   steps 2-5 each inside an `or_else ( | |` and step 1 inside `fill . then ( | |` (lazy ; `then_some (` is refused) ; E4
//!   `wide_name_bits` passed to `fold_self_bits` here only ; E5 the `had_error` readers ; E6 `unwrap_or ( ( 0 , false ) )` ; E7 the
//!   router's call sites (`const_range_bound_fold`, `check_const_range_bound`, `range_bounds_checked`, `range_bound_fold_ev`,
//!   `bound_step`) per (file, fn), and in `range_bounds_checked` the four calls, each its own statement (eager), in PRE's order (fold msb,
//!   fold lsb, check msb, check lsb).
//! REVIEW-ONLY (no test sees them): an entry reached through a fn value or a macro ; a chain spelled with `match` / `if let` instead of
//!   `or_else` ; a decline read after a `let` ; the predicate readers (`.is_some()` over an entry, 23 sites) ; every chain not listed ;
//!   a live-scope REG ask spelled through `fold_bits_at(e, 0, ..)` or a `param_leaf_bits` closure (E4 does not see it ; E2's
//!   fold_bits_at / fold_self_bits counts do) ; a class parameter's default (bound by the parser's specialization, not by a binder here) ;
//!   the router reached through a fn value or a macro ; a declared bound folded under a `let` or another field name ; a fold-then-check
//!   spelled with range_facts and a check ; which site checks a declaration first (by reading only: the val3 lane's site column).

pub(crate) mod arms;
pub(crate) mod chains;
pub(crate) mod lanes;
