# soundness lens r1 — s587 (unique-const-fn)

status: started

## S1 opt-in census (grep const_required(|const_dim_size_fold(|const_held( in wt, excl const_site.rs) = 66 lines
- list = P2 W1-W6 + call-site table + IMPL deviations (params.rs:1239 unfoldable_reason, wide_param_range.rs:69,
  param_decl_range_opt x3, wide_param_decl_range params.rs:2366, instance_array.rs:116 const_held); elab_task :92/:95 dropped (D3).
- const_site writers: only const_site.rs:86/:89/:96/:98 + driver.rs:60 init; reader: const_fn.rs:1734 only.
## S3 interior mutability census: Elaborator Cell/RefCell fields = const_call_fn, const_site, const_call_pkg only
  (lib.rs struct). const_call_* saved/restored at const_fn.rs:1500/1527, const_bound.rs:424/433. No memo in &self path.
  => a Required closure cannot persist state; results leave only by return value to the opted-in consumer.
## S4 catch_unwind: only sim-engine/src/native/kernel_tests.rs:1659 (test); cli/src/pipeline.rs:41 resume_unwind
  re-raises the worker-thread panic (process exits 101). No Elaborator reuse after unwind.
## S2 wrapper callers (grep in wt): W1 const_range_bound_fold 58 call lines, all fold ast::Range (hdl-ast: declared
  packed/unpacked/return/enum-base ranges only; value ranges are RangeEnd). W3 cast_size_bits 9 callers (size of a cast =
  casting_type constant_primary everywhere). W4 2 callers generate.rs:313/:616. W5 5 callers (+: widths expr_main.rs:846/:882,
  lvalue.rs:484/:503; non-string replication expr_main.rs:1052 — string replicate exits at :933 Unstated). W6 3 callers
  ([m:l] only: expr_main.rs:779, lvalue.rs:451, packed.rs:1826<-expr_main.rs:747/lvalue.rs:414). packed_outer/inner_part_select
  callers only try_packed_part_select(_lval) ([m:l]). No run-time caller found.
## S5 producer: hdl-parser assertions.rs:493 only (if-else tail and unique/priority case default); user `$__vita_` refused at
  expr.rs:660 (task) and expr_primary.rs:186 (function). exec_const_stmt has no Stmt::Case arm (case -> None).
  steps: `*steps += 1` per exec_const_stmt call (const_fn.rs:1609), MAX_STEPS 100_000 -> the arm costs one step the plain-if path does not.
## open: cross-scope W1 folds (package fn return/local/formal ranges at module prefix); ia prepass shadow; step edge; pkg range
## correction: hdl-parser lib.rs:889-893 — inside a subroutine body only a LONE `unique if` (first-if-only) is armed;
  my first pk*/iash/pkrng cells used `unique if … else if` = unarmed (= plain). Redone as lone `unique if`.
  pre-existing (PRE=POST, plain twin): package fn return/local/formal range `[f(2):0]` folds `f` in the CALLER's scope
  (pkret0: E3009 "[in top]" without a top f; pkk_p: P=232 B=8 v=232 vs verilator P=8 B=4 v=8).
## FINDING F1 (BLOCKING, NEW loud->silent-wrong): package fn declared ranges fold callee text in the CALLER scope; W1 Required
  pkret_u (lone unique f in top, q::h return [f(2):0], q::f=3): PRE rc=1 E3009 x2 | POST rc=0 P=232 B=8 v=232 | verilator P=8 B=4 v=8
  pkloc_u (local logic [f(2):0] t): PRE rc=1 E3009 [in top.$func$q::h] | POST P=1000 v=232 | verilator P=8 v=8
  pkfml_u (formal input logic [f(2):0] x): PRE rc=1 E3009 | POST P=1000 v=232 | verilator P=8 v=8
  plain twins: PRE=POST P=232/1000 v=232 (pre-existing silent-wrong); iverilog = verilator P=8.
  F1 also W3: pkcast_u (q::h returns f(2)'(x)): PRE rc=1 E3009 "size cast width must be a positive constant" [in top.$func$q::h]
  | POST rc=0 P=0 v=-24 | verilator P=0 v=0 ; plain twin PRE=POST P=0 v=-24 (pre-existing). pkunp (int t[f(2)]) loud everywhere.
  pkrng_u (package param range via r::f): PRE rc=1 E3009 [in q] | POST b=8 = verilator b=8 -> W1 Required in the package binder:
  const_site.rs doc "a package parameter ... states nothing" is false for its declared range (direction correct) -> NIT.
  iash_u (ia child header default q::g lone unique, parent W=8): PRE=POST rc=1 E3009 x2 (Held residue, loud); plain PRE=POST = oracles.
## FINDING F2 (NIT): step edge. steps_u n=22000: PRE/POST E3009; n=19000: POST P=19000; plain n=22000 PRE=POST P=22000 = iverilog
  (verilator refuses both: constant-eval limit). The arm statement costs one step (const_fn.rs:1609) -> loud earlier than plain.
  F1 realistic two-package collision pkimp_u (top imports a::*, a::w lone unique; q::h return [w(2):0], q::w=3):
  PRE rc=1 E3009 x2 [in top] | POST rc=0 P=232 v=232 | verilator P=8 v=8 ; plain twin PRE=POST P=232 v=232; iverilog P=8 v=8.
  inst (regular instance; child port/net range [f(2):0], parent same-named f): PRE=POST=oracles (bp=4 hb=4) -> F1 is the
  inlined package-function scope (`top.$func$q::h`) + package-function range folds, not instance ports.
  prep (process replication {f(2){1'b1}}): PRE y=00000000 (silent-wrong) | POST y=0000007f = verilator/iverilog; no W4031 either.
  nest (repeat/#( f(2)'(g(3)) ), g lone unique miss at run time): POST_u W4031=2 cnt=3 t=3 = verilator (2 violations, cnt=3 t=3);
  PRE_u rc=1 E3009 x2 "size cast width"; plain PRE=POST W4031=2. Required nested in Unstated keeps run-time reports.
## S1 reachability (call graph $S/cg.json, name-keyed, PRE): forward closure of every fold called inside a const_required
  closure = 1599 fns; enclosing fns of the 32 RT/MIXED/INST census rows reachable: 0 ($S/r1/sound/fwd.py).
## S7 Held: window = bind_params(child, overrides) + child port-range folds + restore_params (instance_array.rs:116-148);
  parent instance-array range folds at :40 outside it; overrides = deferred channel (Unstated everywhere). Held reads as
  Unstated at the only reader (const_fn.rs:1734 `== Required`) => PRE-identical inside (iash_u PRE=POST). ifleak (interface fn
  vs parent same-named fn): POST_u P=3 bw=4 bs=8 = verilator; PRE_u loud -> no leak.
  But Held's premise "the instance-array prepass is the pass that folds ANOTHER unit's text" is incomplete: F1.
## S6 text
  N1 NIT: const_site.rs:50-52 "a package parameter ... states nothing"; CHANGELOG + 006 "a package parameter ... stay VITA-E3009"
     -> false for its declared range (package.rs:572 check_param_decl_range -> params.rs:1697 W1); pkrng_u POST b=8 (= verilator).
  N2 NIT: scratch name in code comments: const_site.rs:19, :67, elab_task.rs:26 ("s587 measured …") -> cite §4.5.587.
  N3 NIT (docs commit): docs/REMAINING_WORK.md:12 still lists §3.b `unique-const-fn` as remaining; ROADMAP.md:548/:648 rows.
  checked true: 3 eval_const_call entries (const_fn.rs:514/:520/:1024); `$__vita_` refused expr.rs:660/expr_primary.rs:186;
     no Stmt::Case arm; unique0/priority0 unarmed (assertions.rs); restore-after-f; no catch_unwind.
## VERDICT: FAIL — F1 BLOCKING (new loud->silent-wrong, 5 cells: pkret, pkloc, pkfml, pkcast, pkimp).
