# §4.5.583 differential lens — round 1

PRE  = S/pre/vita  md5 2492e1c5c52c7081a6c0997fe194af40 (release)
POST = S/post/vita md5 41566ca6a6c8eaf9a232bf5741a7058c (release)
PRE-jit bdac9fa485bd34ff2140bcd4d4b44eea, POST-jit 9d508f2d04a2938f7fd09dfa218c28f3
Oracle: verilator 5.052 --binary --timing --assert (+verilator+error+limit+1000). iverilog rejects `unique if`; sv2v never reports.
Cells: S/r1/diff/cells/  (outputs <cell>.out next to each)

## Status
- [x] census: Stmt::If built by parse_if (stmt_ctl.rs:83) and by the immediate assert/assume desugar (assertions.rs:298, else always Some); udp/udp_table/monomorph/type_param_shape not reachable as a statement `else` at parse time
- [x] cells outside the plan table (c01..c19, see table)
- [x] fvoid / class-method narrowing judgement (F3)
- [x] JIT: J1 POST-jit templates_compiled=1 activations=4, reports t1,t3 at 2:33 (PRE-jit on J1_H: t1,t3 at 2:52) = verilator times. F1 reproduces under POST-jit. Staged (vrun subcommands) W4031+stdout == one-shot on every cell that elaborates

## Findings (most severe first)

### F1 BLOCKING — spurious W4031 when the chain's `else` is an immediate `assert`/`assume` whose fail action is an `if` (NEW in POST)
cell: cells/c01_assert_else_if.sv (+ .out). Mechanism: `parse_assert` desugars a plain immediate assert to `Stmt::If{cond, then_s, else_s: Some(fail)}`
(assertions.rs:298); the walk follows `else_s` while it is "directly a Stmt::If", so it enters the assert's If, then its user fail action
`if (c) …` (no else), and arms THAT with the outer span. IEEE §12.4.2: the outer series ends at the `else <assert statement>` — no violation possible.
- verilator 5.052 (raw): no W/Error line at t1..t6, only the six `t=N …` lines.
- PRE native=interp=vm: silent, values identical to verilator.
- POST native=interp=vm (+staged identical W4031/stdout):
  `c01_assert_else_if.sv:5:15: warning[VITA-W4031] … [at time 1]` (unique, assert fails, c=0)
  `c01_assert_else_if.sv:9:22: … [at time 3]` (assume), `:11:15 … [at time 4]` (assert with pass action), `:13:17 … [at time 5]` (priority)
- class: real gap (vita-wrong vs verilator AND hand-IEEE). NEW in POST. Values/print order unchanged; exit 0 both.

- -Werror=W4031: PRE rc=0, POST rc=1 (`error[VITA-W4031] … [at time 1]`) — exit class changes on a design with no violation.
- -Wno-W4031 suppresses it (0 lines). run.json routes PRE==POST.

### F2 BLOCKING (decision input) — time-0 W4031 in the canonical TB/DUT shape, whatever the textual order (NEW instances of the pre-existing 🆕 Z root)
cells: cells/c23_tb_dut_chain_only.sv (TB module written FIRST, DUT `always_comb begin y = 0; unique if (a) y = 1; else if (b) y = 2; end`),
cells/c22_tb_dut_t0.sv (DUT first), cells/c22b_tb_dut_t0_topfirst.sv (TB first), trace wt/c22t.sv.
- verilator (raw, c23): first lines `t=1 y=2`, then `[2] %Error: …:16: … 'unique if' statement violated` (t2 = a/b overlap, out of scope) — nothing at t0.
- iverilog (case analog c22c, raw): `t=1 y=2 z=1` / `WARNING: c22c_case_only.sv:8: value is unhandled … Time: 2  Scope: top.u` — nothing at t0.
- PRE (c23): silent at t0. PRE (c22): `…:8:12: warning[VITA-W4031] … [in top.u] [at time 0]` for `unique case` (root pre-existing).
- POST (c23): `c23_tb_dut_chain_only.sv:16:12: warning[VITA-W4031] … [in top.u] [at time 0]`; -Werror=W4031: PRE rc=0, POST rc=1.
- trace (POST, TB first): `initial start t=0` then `comb eval t=0 a=x b=x` then the report, then `comb eval t=0 a=0 b=1`: the initial runs first; the
  DUT's implicit time-0 always_comb run reads its input PORTS before the port connection propagates. The new manual 006 §1.4 sentence
  ("an always_comb … written before the initial that drives its inputs runs first") is falsified by c22b/c23: TB written first, still reports.
- class: real gap (vita-wrong vs both tools at t0, order). NEW in POST for chains (PRE silent at t0); root pre-existing (PRE does it for unique case).
  Radius: every always_comb/always_latch `unique`/`priority if` chain without final else in an instantiated module whose inputs are x until a t0 initial.

### F3 NON-BLOCKING — narrowing covers every function body (void, class method, ctor, interface, package)
cells: c11b, c18, c21, c05 (fl). verilator reports in all (raw: `… Assertion failed in top.fv`, `$unit.K.m`, `$unit.K.new`, `top.i.f`, `pk.pf`, `top.fi`);
PRE silent, POST silent (no descent; documented). Judgement in the final return.
- PRE-H (armed tree) c18: `[in top.fv] [at time 1]`, `[in top.K.m] [at time 2]` — runs, no refusal.
- PRE-H c11b: `error[VITA-E3009] … package-scoped call pk::pf(...) needs a body that references only its own formals/locals …` — package functions must stay narrowed.
- c09p/c09 H (void call inside a constant fn): verilator `Location of non-constant TASKREF 'g'`; vita PRE=POST=PRE-H E3009 (frame-call subset) — void never reaches const-eval.
- c10: vita refuses `static` class members at parse (E2002) — no class method can be named in a constant expression.

### F4 NON-BLOCKING (pre-existing convention) — scope labels `[in top.pta]`, `[in top.ut]`, `[in top.K.tm]` vs verilator `pk.pta`, `$unit.ut`, `$unit.K.tm`; PRE-H identical.

## Cell table (outside the plan) — verilator raw vs PRE vs POST (native=interp=vm unless noted)
| cell | verilator | PRE | POST | class / new? |
|---|---|---|---|---|
| c01 assert/assume fail action is `if` | silent t1..t6 | silent | W4031 t1,t3,t4,t5 | F1 real gap, NEW |
| c02 `else (* mark *) if`, `unique (* mark *) if` | :5 t1, :7 t2 | silent | 5:15 t1, 7:26 t2 | agree (fixed) |
| c03b call conditions with $display/n++ | t1 report, n=3; hit n=5 (one-hot re-eval, not a count oracle) | n=3/n=2, no report | 6:15 t1, n=3/n=2; PRE-H 6:63, n identical | agree; eval counts PRE=PRE-H=POST |
| c04 generate-for always, interface task, program initial | t2 gen[0],gen[1] :20; t3 top.p :10; t4 top.i.t :4 (+ t0 gen reports = verilator start-up eval, no event at t0) | silent | same lines/scopes/times (no t0) | agree; t0 = verilator not an event-order oracle |
| c05 50-long chain (always_comb), priority 50 chain, function 50 chain | :4 at t2,t4,t7 (+re-evals); function :3 | silent | 4:28 t2,t4,t7; prio 9:25 t7; function silent | agree; fn = documented narrowing |
| c06 recursive auto task, `return`, `disable` in chain | top.rt t1, top.tr t4, top.blk2 t6 | silent | identical lines/scopes/times | agree |
| c07 always_ff NBA chain + unique case in branch + chain in case item + fork | t5 :13,:5; t15 case; priority silent | t15 case only | t5 5:12,13:20; t15; t38 prio 26:25 | agree (same-time order between two processes not an oracle; prio hand-IEEE) |
| c08 x/z conditions | t1 report (2-state) | silent | t1 (x,0), t2 prio (z,x) | agree / hand-IEEE §12.4 |
| c15 `if (b) assert (c); else …` binding, deferred assert in else, assert-else-unique | t1 report :5, t4 :11 | t4 only | t1 5:15, t4 11:64 | agree (verilator binds else to the assert too) |
| c17 imported package tasks | pk.pta :5 t1; prio silent | silent | 5:12 [in top.pta], 8:14 [in top.pts] | agree on line/time; scope `top.pta` vs `pk.pta` is PRE's convention (PRE-H same) |
| c19 $assertoff, else case-inside / do-while / for | :5 t1, :7 t2 (ignores $assertoff) | t2 | t1 5:27, t2 | agree |
| c16/c18b `pk::t(...)` statement | runs | E2002 parse | E2002 parse | pre-existing, identical |
| c18 class fn / class task / module void fn | top.fv, $unit.K.m, $unit.K.tm | silent | K.tm only (7:12) | F3 narrowing; task agree |
| c20 hierarchical task, $unit task, timed task, generate-block task | top.u.t t1, $unit.ut t3, top.u.tt t4, gen t6 | silent | identical lines/times; scopes = PRE-H | agree |
| c21 interface fn, class ctor | top.i.f, $unit.K.new | silent | silent | F3 |
| c22/c22b/c23 TB/DUT t0 | silent t0 | case-only t0 (c22) | chain t0 | F2 |
| J1 under JIT | t1, t3 | PRE-jit H: t1,t3 2:52 | POST-jit t1,t3 2:33, templates_compiled=1 | agree |

## Lanes
native=interp=vm on every cell (c03b's native line is a W4030 S3b fallback, PRE identical). Staged vcmp/velab/vrun W4031+stdout == one-shot on every elaborating cell.
run.json (subroutines/native/backend/codegen/status/exit_code) PRE==POST on c01,c03b,c04,c05,c06,c07,c17,c18,c19,J1.
