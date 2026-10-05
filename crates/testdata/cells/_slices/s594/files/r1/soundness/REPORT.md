# s594 r1 soundness lens REPORT (in progress)

round: 1
status: started

## Q1 const_ctx_within_i64(_context) per-descendant roots (refusal flip hypothesis)
cells W1 W1L W1s W2 W3 W5 W6 W8 W7 (>64-bit name-count replicate in a self-determined / cast position; module, literal twin, 64-bit twin, no-fill, comparison operand, package, generate, header, override-cast):
PRE = POST = iverilog = verilator in all 9 (L=23; W7 P=65535 B=16; sv2v W7 B=8). Hypothesis REFUTED: no descent.

## Q2/Q3 predicate completeness + element type sources
R1 (call under MinTypMax `(1:fl(2):3)` in a shift count, region with `{N{1'b0}}`): PRE=POST E3009 (no fold arm) -> hole unreachable.
R2 (call inside Concat / inner Replicate): PRE=POST E3009 (concat has no fold arm) -> unreachable in this lane.
E1 (type-parameter element, per-instance T override): PRE=POST `u1 L=1 u2 L=1 u3 L=0`; verilator `u2 L=0`, sv2v `u2 L=0` -> PRE-EXISTING (capture uses the default T), not moved.

## Q4 FINDING F1 (BLOCKING candidate): a comparison nested in a call/envw region takes LeafRule::Scope
comparison arm `RegionRule::new(e)` (const_fn_width.rs:613) roots the rule at the comparison node, so a call / never-assigned
local OUTSIDE the comparison no longer keeps PRE's leaf rules (the delta-limiter's stated scope):
C0 `8'd0 + ((X+{N{1'b0}})==8'hFC)` PRE 1 POST 1 ivl 1 vl 1 s2v 1 (control, no call)
C1 `localparam int L = fl(0) + ((X+{N{1'b0}})==8'hFC)` PRE 0 POST 1 | ivl 0 | vl x | s2v x  -> OK->WRONG (plan's own Kfn_cpR standard)
C2 `if ((fl(0) + ((X+{N{1'b0}})==8'hFC)) > 0)` PRE else POST then | ivl else | s2v else | vl then -> OK->WRONG (2 oracles)
C3 `fk = r + ((X+{N{1'b0}})==8'hFC)` (r never assigned) PRE 0 POST 1 | ivl 0 | vl x | s2v x -> OK->WRONG
C3c same, r = 8'd0 assigned: PRE 0 POST 1 | all 3 oracles 1 -> attribution: slice's comparison fix + AE cancellation
C4 repeat(fl(0) + cmp): PRE=POST k=0 (ivl/s2v hang; vl 1) no move.
C6 `fl(0) + $clog2((X+{N{1'b0}})+8'd1)`: PRE=POST L=8 (ivl 0, vl/s2v x) no move.
C8 `localparam int L = fl(0) + (8'd1 << ((X+{N{1'b0}}) >> 7))`: PRE `error[VITA-E3009] ... the `+` operation has no constant-fold arm` POST L=2 | ivl 0 | vl x | s2v x -> LOUD->WRONG
   (K twin without call: POST K=2 = 3 oracles 2.)
C9 `logic [fl(0) + ((X+{N{1'b0}})==8'hFC) + 3 : 0] v`: PRE vb=4 POST vb=5 | ivl vb=1 | s2v vb=x | vl %Error two-state -> WRONG->WRONG'
## Q5 resolver refactor: old closure body vs const_placement_name body (whitespace-stripped, 72 lines each): IDENTICAL; env/envw threaded unchanged -> byte-identical for old callers.

## Census (POST wt @ 1da347bc+diff) — region root each consumer asks under
const_self_width: const_bound.rs:128 tier-3 (root rr, D2) | const_fn_width.rs:1049 eval_const_assign(rhs) | :616/617 comparison (rr = comparison node -> F1 door) | :721 reduction (only reached when operand names env/envw -> Literal by construction) | :775 leaf arm (own root; ctx sized only if enclosing rule sized; ctx_signed is AND under enclosing rule -> OK by argument) | :810 eval_const_env_self (own root -> F1 door, C8) | :895 shift count (names gate) | const_fn.rs:677 const_unsigned_selfdet | const_wide.rs:1697 override_bits fill (UNVERIFIED, plan says unreachable) | param_query.rs:590 override_self_meta (envw = declared map -> any name => Literal) | :811/:851 within_i64 (per-descendant roots; W cells no move) | :900 untyped_fill_init | :978 param_init_kept_loud | params.rs:632/:695 (root = peeled initializer)
const_signed_env: const_fn.rs:562 (same root as :561) | :721 (same root as eval_const_assign(operand)) | param_query.rs:599 (same root/envw as :590) | instance.rs:1303 (syntactically-evident trees only) | instance.rs:1514/1668, iface_inst.rs:32/121 (parent scope, value also parent) | params.rs:128 (same root/envw as width)
const_expr_signed: param_query.rs:903 (same root as :900) | params.rs:130/768/799 | const_fn_width.rs:364 (PkgScoped leaf, no BitSelect)
D1: const_call_pkg set only in eval_const_call (+ const_bound.rs:430 width-safety probe); args folded before the switch; imported fns carry const_fn_pkg -> Literal; $unit fns pkg None, env not seeded -> width twin and value fold share lookup_scoped (no D1 analog; UNVERIFIED by cell).
MINOR M1: const_leaf_rule.rs doc says an envw entry is "a constant-function formal, local or return variable"; in override_self_meta (param_query.rs:590) and params.rs declared_only lane envw = declared_override_widths(e) (every bare NAME) -> any named region is Literal there (conservative; stated reason wrong).

## Mutants (own copy $S/s594/r1/soundness/mut/src, CARGO_TARGET_DIR=$S/s594/target_rv_soundness, `cargo nextest run --workspace --locked --no-fail-fast`)
M0 baseline: 9167 run, 9165 passed, 2 failed (copy-environment: corpus-runner every_uncommitted_manifest_path_is_gitignored, sim-engine s2_admission_census_on_the_hot_design — bench/ excluded from the copy)
Mx1 leaf arm (const_fn_width.rs:773) keeps PRE's leaf rules for a top-level whole element / name-count replicate: 9165 passed, same 2 env failures -> SURVIVED (--workspace). Not equivalent: see X1.
X1/X2 PRE/POST/Mx1 recorded
Mx1 discriminators: X1 module typed `16'd0 + AS[0]` PRE=POST=Mx1 L=252 M=1 (vl/s2v same); X2 function lane: E3009 on PRE/POST/Mx1. Equivalence UNVERIFIED (no separating cell found).
cleanup: target_rv_soundness and mut/src deleted by literal path.
VERDICT: FAIL (F1 BLOCKING: C1 C2 C3 OK->WRONG, C8 LOUD->WRONG, C9 WRONG->WRONG').
