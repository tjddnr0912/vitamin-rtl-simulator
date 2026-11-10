(§5.2 VAL-1 · rank — F · source W/program/reorder2-2026-11-09/PLAN.txt G1 VAL, G2 VAL-1) (the elaborate constant evaluators' shared arms, written once):
* slot: R3.1 · L1, after SYS-1 and PARSE-1 (owner, 2026-11-09: Q-VALRULES (b) becomes VAL-1, VAL-2 and VAL-3 and no longer waits on row 74)
* scope: structure only: every output equals PRE (values, diagnostics, exit code, `.vu` and `.velab` bytes, termination, cost), PRE's differences kept as named lanes (owner, 2026-11-09)
* scope: VAL, the elaboration-time integral constant evaluator: the IEEE value of an integral constant expression at elaboration (parameter binding, bounds, generate conditions, constant-function calls)
* evidence: five cores in four result types: `const_eval_in_scope` (`const_fn.rs:267`, 287 lines) `Option<i64>`, module / instance scope; `eval_const_env` (`:919`, 233 lines) `Option<i64>` with the constant-function frame, `MAX_DEPTH` 64 (`:1607`)
* evidence: `fold_ctx` / `fold_self` / `fold_assign` / `fold_shift_count` (`const_fn_width/fold.rs:166`, 521 lines) `CFold {Val, Four{v,x,why}, No}`, width-aware i64 with an x mask
* evidence: `fold_region` with `fold_bits_at` / `_at0` / `fold_self_bits` / `fold_selfdet_operand` (`const_wide.rs:512`, 590 lines, a free fn naming through a `WideNameFn` callback from 7 files) `Option<WideBits>`, 4-state at any width
* evidence: `env_fold` (`expr_size_hier.rs:692`, 74 lines) `Option<(i64, bool)>`, a strict subset over a child instance's environment
* evidence: with the self-determined adapters, the arms, the constant-function interpreter and the carrier entries (`const_eval_u32`, `const_bound.rs`'s funnel, the router `const_range_bound_fold` → `range_bound_fold_ev`, the delay readers): 357 sites in 49 files
* note: one home is right: the five cores answer one question in one phase and crate, their result types are PRE's lanes of one contract and their scopes an input; they share `const_binop` and overlap in arms
* evidence: ExprKind arms in_scope 17, env 14, region 15, ctx 6, env_fold 7; the `$clog2` arm in all five, three formulas
* fix: each rule two or more cores spell, once in `value/arms.rs` with the core's lane as a parameter: the `$clog2` arm (`const_fn.rs:421` `:1027`, `fold.rs:652`, `expr_size_hier.rs:748`, `const_wide.rs:1053`), the cast arm, the unary / reduction arms (10 unops in 4 cores), concat placement
* fix: the system-call admission per core, read from SYS-1's columns (in_scope `$bits` `$clog2` `$rtoi`, the dimension queries and string methods; env, ctx, env_fold `$clog2`; region `$bits` `$clog2` `$countones` `$isunknown` `$onehot` `$onehot0` `$signed` `$unsigned`)
* note: lanes Unlimited, Env, Ctx, Wide, Hier (one enum, named apart from `range_rules`' `Lane`); an arm runs exactly where the core ran its inline code; `fold_region` stays free (arms are free fns over operand values, scope only through its callback)
* pins: no folded-name arm in a core outside `arms.rs`; ratchet a stays (arms match one kind)
* note: size 5 core files + `arms.rs`; risk medium: a per-lane difference hidden in an arm, so each lane's edge is measured in its grounding (U1: `$clog2` of a negative, a cast of an x)
* note: sibling domains not moved, one file each already: real (`const_real.rs`) and string (`const_str.rs`), whose twins elsewhere are not re-censused (U7)
* note: it frees (C)'s arm once (§5.2 Q-SYMMIX) and Q-PRIMCAST's and Q-XCORE's arms once
* note: the home rows' trip wires and their honest cost are written once, in §5.2 SYS-1
* note: from SYS-1 (§4.5.638, H1): the grounding takes `try_introspect_fold` (`expr_special.rs:598-686`) as a lane beside `const_dim_query` (`const_array.rs:212-260`), both spelling the same query-shape sets (with_dim 6, no_dim 2) inside one crate; the per-core admission columns join `hdl-ast/src/sysfn.rs` (H2)
* note: from PARSE-1 (§4.5.638): the grounding takes `fold_i32`'s 11 computing arms (O1: `const_binop` over its op set, then an i32 cutoff), L14 `const_eval_cast` (its w = 63 dev overflow, §2 dev-overflow-release-wraps), O9's cores, O14 `lowered_truth` and O15 `crv.rs`'s interval arms
* note: E5 (PARSE-1): the note moved with K1–K3 says `3037000500 * 3037000500` is loud too, but f_inscope_ovf (`localparam int Q = 3037000500 * 3037000500;`) prints `L q=145474192` rc 0 on PRE 56c997be, iverilog and verilator alike (W/batch2/docs/probe): the width-aware path answers; this row measures the note
* note: the cost set gains f_szlit, g_sgn and p_sized before this row opens (W/parse1/premise4/RULINGS-main.txt N1)
* source: W/program/reorder2-2026-11-09/PLAN.txt G1 VAL, G2 VAL-1
