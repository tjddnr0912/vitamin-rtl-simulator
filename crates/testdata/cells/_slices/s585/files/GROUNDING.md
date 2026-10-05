# §4.5.585 grounding — unique-if-chain (§5.2 row 1)

HEAD 75f255d4. S = scratchpad. PRE = S/s585/pre/vita md5 c766b8d59edabaef96436e7b46e75dc6 (release, 75f255d4).
Harness: S/s585/g/r.py (verilator 5.052 --binary --timing --assert +verilator+error+limit+1000; iverilog 13 -g2012;
sv2v 0.0.13 = S/s580/sv2v/sv2v-macOS/sv2v -> iverilog; vita one-shot `vita --backend X file.sv`).
Cells: S/s585/g/c/*.sv, outputs S/s585/g/out/*.out.

## Status
- [x] Q1 F2 re-measure on PRE
- [x] Q2 written-token census + mechanism
- [x] Q3 function-body narrowing census + cells
- [~] Q4 POST-cand (partial: STOPPED, see Stop)
- [ ] Q5 lanes

## Q1 — F2 shapes on PRE (75f255d4)

Cells: q1_<form>_<proc>_<order> (DUT through ports; form chain|H|lone|case; proc always_comb|always_latch; tbF = TB module
written first, tbL = last); q1d_<form>_<order> (TB -> mid (producer always_comb) -> leaf DUT; leafF = leaf written first);
q1ca_<form>_<order> (TB drives DUT ports through `assign` from regs). Stimulus: t0 a=0 b=1 (match), t2 b=0 (real no-match).
Column format: tool:time@line[:col]. vl = verilator, iv = iverilog (REJ = rejects `unique if`), PRE = native=interp=vm unless marked.
verilator re-reports at t3/t4 = its re-evaluation (not an event-count oracle); vl c22/c22b/c23 t2 at the `unique if` line includes the
a=b=1 OVERLAP report (multi-match, out of scope).

```
q1_H_comb_tbF | vl:2@16,3@16,4@16 | iv:REJ | PRE:2@16:31
q1_H_comb_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:2@4:31
q1_H_latch_tbF | vl:2@16,3@16,4@16 | iv:REJ | PRE:2@16:31
q1_H_latch_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:2@4:31
q1_case_comb_tbF | vl:2@16,3@16,4@16 | iv:2@16 | PRE:2@16:12
q1_case_comb_tbL | vl:2@4,3@4,4@4 | iv:2@4 | PRE:2@4:12
q1_case_latch_tbF | vl:2@16,3@16,4@16 | iv:2@16 | PRE:2@16:12
q1_case_latch_tbL | vl:2@4,3@4,4@4 | iv:2@4 | PRE:2@4:12
q1_chain_comb_tbF | vl:2@16,3@16,4@16 | iv:REJ | PRE:-
q1_chain_comb_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:-
q1_chain_latch_tbF | vl:2@16,3@16,4@16 | iv:REJ | PRE:-
q1_chain_latch_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:-
q1_lone_comb_tbF | vl:2@16,3@16,4@16 | iv:REJ | PRE:2@16:12
q1_lone_comb_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:2@4:12
q1_lone_latch_tbF | vl:2@16,3@16,4@16 | iv:REJ | PRE:2@16:12
q1_lone_latch_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:2@4:12
q1d_H_leafF | vl:2@4,3@4,4@4 | iv:REJ | PRE:2@4:31
q1d_H_tbF | vl:2@21,3@21,4@21 | iv:REJ | PRE:2@21:31
q1d_case_leafF | vl:2@4,3@4,4@4 | iv:2@4 | PRE:2@4:12
q1d_case_tbF | vl:2@21,3@21,4@21 | iv:2@21 | PRE:2@21:12
q1d_chain_leafF | vl:2@4,3@4,4@4 | iv:REJ | PRE:-
q1d_chain_tbF | vl:2@21,3@21,4@21 | iv:REJ | PRE:-
q1d_lone_leafF | vl:2@4,3@4,4@4 | iv:REJ | PRE:2@4:12
q1d_lone_tbF | vl:2@21,3@21,4@21 | iv:REJ | PRE:2@21:12
q1ca_H_tbF | vl:2@17,3@17,4@17 | iv:REJ | PRE:2@17:31
q1ca_H_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:2@4:31
q1ca_case_tbF | vl:2@17,3@17,4@17 | iv:2@17 | PRE:2@17:12
q1ca_case_tbL | vl:2@4,3@4,4@4 | iv:2@4 | PRE:2@4:12
q1ca_chain_tbF | vl:2@17,3@17,4@17 | iv:REJ | PRE:-
q1ca_chain_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:-
q1ca_lone_tbF | vl:2@17,3@17,4@17 | iv:REJ | PRE:2@17:12
q1ca_lone_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:2@4:12
c22_tb_dut_t0 | vl:2@8,2@4,3@8,3@4,4@8,4@4 | iv:REJ | PRE:2@8:12
c22b_tb_dut_t0_topfirst | vl:2@20,2@16,3@20,3@16,4@20,4@16 | iv:REJ | PRE:2@20:12
c22c_case_only | vl:2@8,3@8,4@8 | iv:2@8 | PRE:2@8:12
c23_tb_dut_chain_only | vl:2@16,3@16,4@16 | iv:REJ | PRE:-
b03 | vl:2@6,3@6,4@6 | iv:REJ | PRE:-
b03H | vl:2@6,3@6,4@6 | iv:REJ | PRE:2@6:31,3@6:31
b04_single_comb_t0 | vl:2@6,3@6,4@6 | iv:REJ | PRE:2@6:12,3@6:12
a00_repro | vl:2@5,3@5,4@5 | iv:2@5 | PRE:2@5:12
a08_comb_chain | vl:- | iv:5@6 | PRE:0@6:12,5@6:12
a08b_comb_chain_rev | vl:- | iv:0@7 | PRE:-
c04_gen_intf_prog | vl:0@20,2@20,3@10,4@4 | iv:REJ | PRE:-
```
Q1 verdict: 0 t0 W4031 on PRE in all 38 TB/DUT/port/CA/deep cells, both module orders, comb and latch, lone / case / H.
The only t0 report left in this set is a08_comb_chain (PRE 0@6:12; vl silent; iv silent at t0, reports t5) = 🆕 AA shape
(consumer comb `unique case` written before producer `always_comb b = a`, source = declaration initializer `a = 0`, no t0 process write).
c04 chains are unarmed on PRE (see Q4 for POST).

## Q2 — written-token census (wt-s585 = 75f255d4)

Stmt::If producers in crates/hdl-parser/src (non-test):
- stmt_ctl.rs:83 `parse_if` — the only statement-position producer from a written `if`; callers stmt.rs:153 (`Kw::If`) and
  assertions.rs:479 (`parse_unique_priority`, after peek == `Kw::If`). Span lo = the `if` token (start = cur_span() at stmt_ctl.rs:71).
  `else` parsed at stmt_ctl.rs:78-80: `if self.eat_kw(Kw::Else) { Some(Box::new(self.parse_statement())) }`.
- assertions.rs:298 `parse_assert` plain immediate `assert`/`assume` (stmt.rs:178) -> `Stmt::If{else_s: Some(..)}`, span lo = `assert`/`assume` token (assertions.rs:224).
- Not statement-position / not during parse_unique_priority: type_param_shape.rs:176 (module-item Initial), udp_table.rs:118/126, udp.rs:619-670 (UDP),
  monomorph.rs:194/656 (post-parse AST subst). generate.rs:253 is GenItem::If.
- Wrappers that end a series (not Stmt::If): parse_labeled_stmt stmt.rs:232 (-> Block), parse_seq_block, parse_delay_stmt (`#`), parse_event_stmt (`@`),
  `;` -> Stmt::Null, DeferredAssert (`assert #0/final`), parse_wait, etc.
- Attributes `(* … *)` are removed by the lexer (hdl-lexer/src/lib.rs:739 `strip_attribute_instances`), so `else (* x *) if` reaches parse_if as `else if`.
- One Parser per (tokens, src) (api.rs:27 Parser::new), so a byte offset names one token.

Mechanism (no hdl-ast change, no SchemaHash move): Parser field `else_if_at: BTreeSet<u32>`, init in Parser::new (lib.rs:~950).
At the `else` site in parse_if: `written = at_kw(If) || (peek in {Unique, Priority, Unique0, Priority0} && peek_at(1) == If)`;
after parse_statement, if written and the result is Stmt::If, insert its span.lo. In parse_unique_priority the walk follows `else_s` only
while it is a Stmt::If whose span.lo is in `else_if_at` (positive set). Consumers of the field: parse_if (insert) and parse_unique_priority
(contains) only.

## Q3 — function-body narrowing census + cells

exec_const_stmt (elaborate/src/const_fn.rs:1588) is entered only from eval_const_call (const_fn.rs:1509) and itself; eval_const_call callers
const_fn.rs:514 (`p::f`), :520 (`f`), :1020 (nested). Bodies come from const_fn_def (const_fn.rs:1379): `pkg_funcs` (package.rs:833 -> :1033,
ModuleItem::Func only) and `const_func_table` (instance.rs:527 module body ModuleItem::Func, incl. injected $unit items; iface_inst.rs:232
interface body; package.rs:1695/1730 imports). `function void` -> ModuleItem::Task at parse (module_items.rs:322); class methods ->
ClassItem::Func/Task (classes.rs:285/292), never in either table. Parser: parse_function_def callers = classes.rs:285 (class) and
module_items.rs:321 parse_function_item (<- :616 $unit, :1340 module/interface/package/program/generate items).

Cells S/s585/g/c3/<kind>.sv (plain chain: verilator, PRE) and <kind>_H.sv (H spelling = POST's armed tree on PRE: PRE-H).
`_rt` = no-match reached by a run-time call at t1; `_k` = constant context (localparam / header default / generate condition).
x=0 makes the chain miss; r=7 is the folded/returned value.
```
cls_ctor_rt | vl:1@5 | PRE:- | PRE-H:1@5:36
cls_fn_rt | vl:1@4 | PRE:- | PRE-H:1@4:36
cls_sfn_k | vl:- | PRE:elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:31(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002) rc=1 | PRE-H:elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:24(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002) rc=1
cls_sfn_rt | vl:1@4 | PRE:elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:31(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002) rc=1 | PRE-H:elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:24(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002) rc=1
cls_task_rt | vl:1@5 | PRE:- | PRE-H:1@5:36
cls_vfn_rt | vl:1@5 | PRE:- | PRE-H:1@5:36
cls_vfn_via_f_rt | vl:1@4 | PRE:elab@6:32(VITA-E3010) rc=1 | PRE-H:elab@6:32(VITA-E3010) rc=1
gen_fn_k | vl:COMPILE-FAIL | PRE:elab@8:24(VITA-E3009),elab@9:13(VITA-E3010) rc=1 | PRE-H:elab@8:24(VITA-E3009),elab@9:13(VITA-E3010) rc=1
gen_fn_rt | vl:1@5 | PRE:- | PRE-H:1@5:38
ifc_fn_k | vl:- | PRE:- | PRE-H:elab@7:22(VITA-E3009) rc=1
ifc_fn_rt | vl:1@4 | PRE:- | PRE-H:1@4:36
ifc_vfn_rt | vl:1@4 | PRE:- | PRE-H:1@4:36
m_auto_k | vl:- | PRE:- | PRE-H:elab@7:22(VITA-E3009) rc=1
m_auto_rt | vl:1@4 | PRE:- | PRE-H:1@4:36
m_static_k | vl:- | PRE:- | PRE-H:elab@7:22(VITA-E3009) rc=1
m_static_kgen | vl:- | PRE:- | PRE-H:elab@7:7(VITA-E3010) rc=1
m_static_khdr | vl:- | PRE:- | PRE-H:elab@1:32(VITA-E3009) rc=1
m_static_rt | vl:1@4 | PRE:- | PRE-H:1@4:36
m_vfn_via_f_rt | vl:1@4 | PRE:elab@1:8(VITA-E3009) rc=1 | PRE-H:elab@1:8(VITA-E3009) rc=1
m_vfn2_via_f_k | vl:COMPILE-FAIL | PRE:elab@7:22(VITA-E3009),elab@1:8(VITA-E3009) rc=1 | PRE-H:elab@7:22(VITA-E3009),elab@1:8(VITA-E3009) rc=1
m_vfn2_via_f_rt | vl:1@4 | PRE:elab@1:8(VITA-E3009) rc=1 | PRE-H:elab@1:8(VITA-E3009) rc=1
m_void_k | vl:COMPILE-FAIL | PRE:elab@7:22(VITA-E3009),elab@1:8(VITA-E3009) rc=1 | PRE-H:elab@7:22(VITA-E3009),elab@1:8(VITA-E3009) rc=1
m_void_rt | vl:1@4 | PRE:- | PRE-H:1@4:36
pkg_fn_k | vl:- | PRE:- | PRE-H:elab@9:22(VITA-E3009) rc=1
pkg_fn_kimp | vl:- | PRE:- | PRE-H:elab@10:22(VITA-E3009) rc=1
pkg_fn_rt | vl:1@4 | PRE:- | PRE-H:elab@9:20(VITA-E3009) rc=1
pkg_task_imp_rt | vl:1@8 | PRE:elab@14:8(VITA-E3009) rc=1 | PRE-H:elab@14:8(VITA-E3009) rc=1
pkg_vfn_imp_rt | vl:1@4 | PRE:elab@14:8(VITA-E3009) rc=1 | PRE-H:elab@14:8(VITA-E3009) rc=1
pkg_vfn_via_f_imp_rt | vl:1@4 | PRE:elab@14:8(VITA-E3009) rc=1 | PRE-H:elab@14:8(VITA-E3009) rc=1
pkg_vfn_via_f_k | vl:COMPILE-FAIL | PRE:elab@15:22(VITA-E3009) rc=1 | PRE-H:elab@15:22(VITA-E3009) rc=1
pkg_vfn_via_f_rt | vl:1@4 | PRE:elab@15:20(VITA-E3009) rc=1 | PRE-H:elab@15:20(VITA-E3009) rc=1
pkg_vfn2_imp_rt | vl:1@4 | PRE:elab@10:8(VITA-E3009) rc=1 | PRE-H:elab@10:8(VITA-E3009) rc=1
pkg_vfn2_via_f_imp_rt | vl:1@4 | PRE:elab@10:8(VITA-E3009) rc=1 | PRE-H:elab@10:8(VITA-E3009) rc=1
pkg_vfn2_via_f_rt | vl:1@4 | PRE:elab@11:20(VITA-E3009) rc=1 | PRE-H:elab@11:20(VITA-E3009) rc=1
prog_fn_rt | vl:1@4 | PRE:- | PRE-H:1@4:36
unit_fn_k | vl:- | PRE:- | PRE-H:elab@7:22(VITA-E3009) rc=1
unit_fn_rt | vl:1@3 | PRE:- | PRE-H:1@3:34
pkg_vfn3_imp_rt | vl:1@4,3@8 | PRE:- | PRE-H:1@4:36,3@8:36
```
Per kind (PRE-H E3009 in `_k` = the constant interpreter executes the body and refuses the arm):
| kind | const call executes body? | verilator rt | verilator const | decision |
|---|---|---|---|---|
| module non-void static / automatic | yes (m_static_k_H, m_auto_k_H, khdr_H E3009; kgen_H E3010) | report | P=7 silent | keep first-if (narrow) |
| $unit non-void | yes (unit_fn_k_H E3009) | report | silent | narrow |
| interface non-void | yes (ifc_fn_k_H E3009) | report | silent | narrow |
| package non-void | yes (pkg_fn_k_H, pkg_fn_kimp_H E3009) + run-time pk:: scoped call refuses the armed body (pkg_fn_rt_H E3009) | report | silent | narrow |
| program non-void | not measured in const context; run-time PRE-H runs (prog_fn_rt_H) | report | — | narrow (conservative) |
| generate-block non-void | no: PRE and PRE-H both E3009 in gen_fn_k (refuses with or without the arm); verilator refuses too | report | refuses | narrowed by the flag (parser has no generate context); residue |
| module/interface/$unit/package void (-> Task) | no: a void call in a const fn refuses before the body (m_void_k, m_vfn2_via_f_k PRE = PRE-H E3009; verilator refuses) | report | refuses | arm |
| class function / void / task / ctor | no: static class members are E2002 at parse (cls_sfn_k, cls_sfn_rt), instance methods have no const call | report | P=7 (static) | arm |
Side observation (pre-existing, loud both sides): a non-void function calling a void function is E3009 on PRE (m_vfn_via_f_rt, pkg_vfn*_via_f*);
on pkg_vfn2_via_f_rt / pkg_vfn_via_f_rt the E3009 TEXT changes when the void callee is armed (PRE: `frame function/task pk::f body uses an
assignment to a net outside`, PRE-H: `package-scoped call pk::f(...) reaches pk::fv, whose body names something outside`), rc=1 both.

Narrowing set (flag true) = parse_function_def reached from parse_function_item (module_items.rs:321) with !is_void. Class methods (classes.rs:285),
void functions and all tasks get the chain rule.

## Q4 — POST-cand (partial)

POST-cand = S/s585/postc/vita md5 17b7d988696c0f5b38b7e94b845e2df9 (wt-s585 release), diff = S/s585/postc/cand.patch, `git diff | md5` = 1e8a0e2107f4e6a85b31aa95081a65c4.
Changes vs attempt: `in_function_body` -> `in_const_fn_body` (true only for parse_function_def(item=true) && !is_void), new `else_if_at`
recorded in parse_if's `else` arm, walk keyed on `else_if_at.contains(span.lo)`.
Cells: S/s583/plan/*.sv, S/s583/r1/sound/cells/*.sv (snd_), S/s583/r1/diff/cells/*.sv (dif_), F1 cells S/s585/g/c4/*.sv. All with --staged
(POST and PRE vcmp->velab->vrun; `staged ... False` rows are all elaboration refusals where the staged run stops at vcmp/velab, PRE and POST alike).
```
b02H | vl:1@7,2@13,3@18,4@22 | iv:REJ | PRE:1@7:31,2@13:12,3@18:31,4@22:12 | POST:1@7:31,2@13:12,3@18:31,4@22:12
b03H | vl:2@6,3@6,4@6 | iv:REJ | PRE:2@6:31,3@6:31 | POST:2@6:31,3@6:31
ca_H | vl:1@3,2@4 | iv:REJ | PRE:1@3:61,2@4:62 | POST:1@3:61,2@4:62
dif_c01_assert_else_if | vl:- | iv:REJ | PRE:- | POST:-
dif_c02_attr_else_if | vl:1@5,2@7 | iv:REJ | PRE:- | POST:1@5:15,2@7:26
dif_c03_side_eff_H | vl:1@6,3@10 | iv:REJ | PRE:elab@10:13(VITA-E2002),elab@10:16(VITA-E2002),elab@10:16(VITA-E2002),elab@10:16(VITA-E2002),elab@10:29(VITA-E2002) rc=1 | POST:elab@10:13(VITA-E2002),elab@10:16(VITA-E2002),elab@10:16(VITA-E2002),elab@10:16(VITA-E2002),elab@10:29(VITA-E2002),elab@10:13(VITA-E2002),elab@10:16(VITA-E2002),elab@10:16(VITA-E2002),elab@10:16(VITA-E2002),elab@10:29(VITA-E2002),elab@-:-(VITA-E8005),elab@-:-(VITA-E8005),elab@10:13(VITA-E2002),elab@10:16(VITA-E2002),elab@10:16(VITA-E2002),elab@10:16(VITA-E2002),elab@10:29(VITA-E2002),elab@-:-(VITA-E8005),elab@-:-(VITA-E8005) rc=1
dif_c03_side_eff | vl:1@6 | iv:REJ | PRE:elab@10:20(VITA-E2002),elab@10:23(VITA-E2002),elab@10:23(VITA-E2002),elab@10:23(VITA-E2002),elab@10:36(VITA-E2002) rc=1 | POST:elab@10:20(VITA-E2002),elab@10:23(VITA-E2002),elab@10:23(VITA-E2002),elab@10:23(VITA-E2002),elab@10:36(VITA-E2002),elab@10:20(VITA-E2002),elab@10:23(VITA-E2002),elab@10:23(VITA-E2002),elab@10:23(VITA-E2002),elab@10:36(VITA-E2002),elab@-:-(VITA-E8005),elab@-:-(VITA-E8005),elab@10:20(VITA-E2002),elab@10:23(VITA-E2002),elab@10:23(VITA-E2002),elab@10:23(VITA-E2002),elab@10:36(VITA-E2002),elab@-:-(VITA-E8005),elab@-:-(VITA-E8005) rc=1
dif_c03b_side_eff_H | vl:1@6 | iv:REJ | PRE[native]:1@6:63 | PRE[interp]:1@6:63 | PRE[vm]:1@6:63 | POST[native]:1@6:63 | POST[interp]:1@6:63 | POST[vm]:1@6:63
dif_c03b_side_eff | vl:1@6 | iv:REJ | PRE[native]:- | PRE[interp]:- | PRE[vm]:- | POST[native]:1@6:15 | POST[interp]:1@6:15 | POST[vm]:1@6:15
dif_c04_gen_intf_prog | vl:0@20,2@20,3@10,4@4 | iv:REJ | PRE:- | POST:2@20:14,2@20:14,3@10:15,4@4:12
dif_c05_long50 | vl:2@4,4@4,5@3,5@4,6@4,7@4,8@4 | iv:REJ | PRE:- | POST:2@4:28,4@4:28,7@9:25,7@4:28
dif_c06_rec_ret_dis | vl:1@4,4@9,6@23 | iv:REJ | PRE:- | POST:1@4:12,4@9:12,6@23:14
dif_c07_ff_case_fork | vl:5@13,5@5,15@7 | iv:REJ | PRE:15@7:14 | POST:5@5:12,5@13:20,15@7:14,38@26:25
dif_c08_xz | vl:1@5 | iv:REJ | PRE:- | POST:1@5:35,2@7:37
dif_c09_fvoid_const_H | vl:COMPILE-FAIL | iv:REJ | PRE:elab@9:22(VITA-E3009),elab@1:8(VITA-E3009) rc=1 | POST:elab@9:22(VITA-E3009),elab@1:8(VITA-E3009),elab@9:22(VITA-E3009),elab@1:8(VITA-E3009),elab@-:-(VITA-E8005),elab@9:22(VITA-E3009),elab@1:8(VITA-E3009),elab@-:-(VITA-E8005) rc=1
dif_c09p_fvoid_const | vl:COMPILE-FAIL | iv:REJ | PRE:elab@9:22(VITA-E3009),elab@1:8(VITA-E3009) rc=1 | POST:elab@9:22(VITA-E3009),elab@1:8(VITA-E3009),elab@9:22(VITA-E3009),elab@1:8(VITA-E3009),elab@-:-(VITA-E8005),elab@9:22(VITA-E3009),elab@1:8(VITA-E3009),elab@-:-(VITA-E8005) rc=1
dif_c10_cls_static_const_H | vl:- | iv:REJ | PRE:elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:24(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002) rc=1 | POST:elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:24(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002),elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:24(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002),elab@-:-(VITA-E8005),elab@-:-(VITA-E8005),elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:24(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002),elab@-:-(VITA-E8005),elab@-:-(VITA-E8005) rc=1
dif_c10n_cls_static_const_noarm | vl:- | iv:REJ | PRE:elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002) rc=1 | POST:elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002),elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002),elab@-:-(VITA-E8005),elab@-:-(VITA-E8005),elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002),elab@-:-(VITA-E8005),elab@-:-(VITA-E8005) rc=1
dif_c11_fn_runtime | vl:1@22,2@25,3@4,4@11 | iv:REJ | PRE:elab@13:3(VITA-E2002),elab@14:12(VITA-E2002),elab@15:5(VITA-E2002),elab@15:28(VITA-E2002),elab@16:5(VITA-E2002),elab@17:3(VITA-E2002) rc=1 | POST:elab@13:3(VITA-E2002),elab@14:12(VITA-E2002),elab@15:5(VITA-E2002),elab@15:28(VITA-E2002),elab@16:5(VITA-E2002),elab@17:3(VITA-E2002),elab@13:3(VITA-E2002),elab@14:12(VITA-E2002),elab@15:5(VITA-E2002),elab@15:28(VITA-E2002),elab@16:5(VITA-E2002),elab@17:3(VITA-E2002),elab@-:-(VITA-E8005),elab@-:-(VITA-E8005),elab@13:3(VITA-E2002),elab@14:12(VITA-E2002),elab@15:5(VITA-E2002),elab@15:28(VITA-E2002),elab@16:5(VITA-E2002),elab@17:3(VITA-E2002),elab@-:-(VITA-E8005),elab@-:-(VITA-E8005) rc=1
dif_c11b_fn_runtime_H | vl:1@17,2@21,3@21,4@4,5@11 | iv:REJ | PRE:elab@29:8(VITA-E3009) rc=1 | POST:elab@29:8(VITA-E3009),elab@29:8(VITA-E3009),elab@-:-(VITA-E8005),elab@29:8(VITA-E3009),elab@-:-(VITA-E8005) rc=1
dif_c11b_fn_runtime | vl:1@17,2@21,3@21,4@4,5@11 | iv:REJ | PRE:- | POST:1@17:12,5@11:12
dif_c15_assert_semi_else | vl:1@5,4@11 | iv:REJ | PRE:4@11:64 | POST:1@5:15,4@11:64
dif_c16_pkg_task_scoped | vl:1@5 | iv:REJ | PRE:elab@15:10(VITA-E2002),elab@16:10(VITA-E2002),elab@17:17(VITA-E2002) rc=1 | POST:elab@15:10(VITA-E2002),elab@16:10(VITA-E2002),elab@17:17(VITA-E2002),elab@15:10(VITA-E2002),elab@16:10(VITA-E2002),elab@17:17(VITA-E2002),elab@-:-(VITA-E8005),elab@-:-(VITA-E8005),elab@15:10(VITA-E2002),elab@16:10(VITA-E2002),elab@17:17(VITA-E2002),elab@-:-(VITA-E8005),elab@-:-(VITA-E8005) rc=1
dif_c17_pkg_task_import_H | vl:1@5 | iv:REJ | PRE:1@5:31,2@8:35 | POST:1@5:31,2@8:35
dif_c17_pkg_task_import | vl:1@5 | iv:REJ | PRE:- | POST:1@5:12,2@8:14
dif_c18_cls_void_H | vl:1@13,2@4,3@7 | iv:REJ | PRE:1@13:31,2@4:31,3@7:31 | POST:1@13:31,2@4:31,3@7:31
dif_c18_cls_void | vl:1@13,2@4,3@7 | iv:REJ | PRE:- | POST:1@13:12,2@4:12,3@7:12
dif_c18b_pkg_void_scoped_H | vl:1@4 | iv:REJ | PRE:elab@11:10(VITA-E2002) rc=1 | POST:elab@11:10(VITA-E2002),elab@11:10(VITA-E2002),elab@-:-(VITA-E8005),elab@-:-(VITA-E8005),elab@11:10(VITA-E2002),elab@-:-(VITA-E8005),elab@-:-(VITA-E8005) rc=1
dif_c18b_pkg_void_scoped | vl:1@4 | iv:REJ | PRE:elab@11:10(VITA-E2002) rc=1 | POST:elab@11:10(VITA-E2002),elab@11:10(VITA-E2002),elab@-:-(VITA-E8005),elab@-:-(VITA-E8005),elab@11:10(VITA-E2002),elab@-:-(VITA-E8005),elab@-:-(VITA-E8005) rc=1
dif_c19_assertoff_else_forms | vl:1@5,2@7 | iv:REJ | PRE:2@7:15 | POST:1@5:27,2@7:15
dif_c20_hier_unit_timed_H | vl:1@8,3@3,4@11,6@19 | iv:REJ | PRE:1@8:31,3@3:29,4@11:34,6@19:37 | POST:1@8:31,3@3:29,4@11:34,6@19:37
dif_c20_hier_unit_timed | vl:1@8,3@3,4@11,6@19 | iv:REJ | PRE:- | POST:1@8:12,3@3:10,4@11:12,6@19:14
dif_c21_ctor_ifc_fn | vl:1@5,2@12 | iv:REJ | PRE:- | POST:2@12:12
dif_c22_tb_dut_t0 | vl:2@8,2@4,3@8,3@4,4@8,4@4 | iv:REJ | PRE:2@8:12 | POST:2@4:12,2@8:12
dif_c22b_tb_dut_t0_topfirst | vl:2@20,2@16,3@20,3@16,4@20,4@16 | iv:REJ | PRE:2@20:12 | POST:2@16:12,2@20:12
dif_c22c_case_only | vl:2@8,3@8,4@8 | iv:2@8 | PRE:2@8:12 | POST:2@8:12
dif_c23_tb_dut_chain_only | vl:2@16,3@16,4@16 | iv:REJ | PRE:- | POST:-
dif_J1_H | vl:1@2,3@2 | iv:REJ | PRE:1@2:52,3@2:52 | POST:1@2:52,3@2:52
dif_J1 | vl:1@2,3@2 | iv:REJ | PRE:- | POST:1@2:33,3@2:33
fold_H | vl:0@5 | iv:REJ | PRE:0@5:32,0@5:32,0@5:32 | POST:0@5:32,0@5:32,0@5:32
fold_P | vl:0@5 | iv:REJ | PRE:- | POST:-
l07f_H | vl:1@5,2@5,3@5 | iv:REJ | PRE:0@5:31,1@5:31,1@5:31,2@5:31 | POST:0@5:31,1@5:31,1@5:31,2@5:31
l07f_P | vl:1@5,2@5,3@5 | iv:REJ | PRE:- | POST:-
l07f_S | vl:1@5,2@5,3@5 | iv:REJ | PRE:0@5:12,1@5:12,1@5:12,2@5:12 | POST:0@5:12,1@5:12,1@5:12,2@5:12
l08_Cs | vl:- | iv:- | PRE:elab@6:23(VITA-E3009) rc=1 | POST:elab@6:23(VITA-E3009),elab@6:23(VITA-E3009),elab@-:-(VITA-E8005),elab@6:23(VITA-E3009),elab@-:-(VITA-E8005) rc=1
l08_H | vl:- | iv:REJ | PRE:elab@6:23(VITA-E3009) rc=1 | POST:elab@6:23(VITA-E3009),elab@6:23(VITA-E3009),elab@-:-(VITA-E8005),elab@6:23(VITA-E3009),elab@-:-(VITA-E8005) rc=1
l08_Hm | vl:0@4 | iv:REJ | PRE:0@4:37 | POST:0@4:37
l08_P | vl:- | iv:REJ | PRE:- | POST:-
l08_S | vl:- | iv:REJ | PRE:elab@6:23(VITA-E3009) rc=1 | POST:elab@6:23(VITA-E3009),elab@6:23(VITA-E3009),elab@-:-(VITA-E8005),elab@6:23(VITA-E3009),elab@-:-(VITA-E8005) rc=1
lt_H | vl:1@9,2@2,3@5,4@16,5@19 | iv:REJ | PRE:1@9:97,2@2:97,3@5:76,4@16:54,5@19:50 | POST:1@9:97,2@2:97,3@5:76,4@16:54,5@19:50
lt_P | vl:1@9,2@2,3@5,4@16,5@19 | iv:REJ | PRE:- | POST:1@9:78,2@2:78,3@5:48,4@16:26,5@19:22
snd_c1_assert_if | vl:- | iv:REJ | PRE:- | POST:-
snd_c2a_fv_in_cf | vl:COMPILE-FAIL | iv:REJ | PRE:elab@9:26(VITA-E3009),elab@1:8(VITA-E3009) rc=1 | POST:elab@9:26(VITA-E3009),elab@1:8(VITA-E3009),elab@9:26(VITA-E3009),elab@1:8(VITA-E3009),elab@-:-(VITA-E8005),elab@9:26(VITA-E3009),elab@1:8(VITA-E3009),elab@-:-(VITA-E8005) rc=1
snd_c2b_fv_in_cf | vl:COMPILE-FAIL | iv:REJ | PRE:elab@9:26(VITA-E3009),elab@1:8(VITA-E3009) rc=1 | POST:elab@9:26(VITA-E3009),elab@1:8(VITA-E3009),elab@9:26(VITA-E3009),elab@1:8(VITA-E3009),elab@-:-(VITA-E8005),elab@9:26(VITA-E3009),elab@1:8(VITA-E3009),elab@-:-(VITA-E8005) rc=1
snd_c2c_fv_in_cf | vl:- | iv:REJ | PRE:- | POST:-
snd_c3h_class_fn | vl:1@4,2@8,4@12 | iv:REJ | PRE:1@4:56,2@8:33,4@12:54 | POST:1@4:56,2@8:33,4@12:54
snd_c3p_class_fn | vl:1@4,2@8,4@12 | iv:REJ | PRE:- | POST:1@4:14,2@8:14,4@12:12
snd_c5_attr | vl:1@4 | iv:REJ | PRE:- | POST:1@4:15
snd_c6_assert_default | vl:1@4 | iv:REJ | PRE:1@4:53(VITA-E4003) rc=1 | POST:1@4:53(VITA-E4003) rc=1
snd_c7_chain | vl:1@4 | iv:REJ | PRE:- | POST:1@4:15
snd_c8_deferred | vl:- | iv:REJ | PRE:- | POST:-
vl_lines | vl:1@7,2@11,3@14,4@17,5@21,7@27 | iv:REJ | PRE[native]:2@12:20 | PRE[interp]:2@12:20 | PRE[vm]:2@12:20 | POST[native]:1@7:15,2@12:20,3@14:15,4@18:20,5@21:15,7@27:22 | POST[interp]:1@7:15,2@12:20,3@14:15,4@18:20,5@21:15,7@27:22 | POST[vm]:1@7:15,2@12:20,3@14:15,4@18:20,5@21:15,7@27:22
f1a_assert | vl:8@19 | PRE:8@19:57 | POST:8@19:57
f1b_enders | vl:- | PRE:- | POST:-
f1c2_qual | vl:1@5,2@7,3@9,4@11,6@17,7@19,8@21,9@23,13@31 | PRE:3@9:43,6@17:15 | POST:1@5:15,2@7:15,3@9:43,4@11:15,6@17:15,7@19:15,8@21:41,9@23:15,11@27:17,13@31:15
f1d_prio0 | vl:COMPILE-FAIL | PRE:- | POST:1@5:15,2@7:17
```
(continued below after the judge's ruling)

## Stop — t0 report in a TB/DUT shape that is not 🆕 AA (pre-existing, PRE == POST)

Found from plan cell l07f_S (lone `unique if` in a function, PRE = POST t0 report). Isolated:
```
t0f_both_case | vl:1@5,2@5,3@5 | iv:1@5,1@5,2@5 | PRE:0@5:12,1@5:12,1@5:12,2@5:12 | POST:0@5:12,1@5:12,1@5:12,2@5:12 | PRE==POST (all backends): True
t0f_ca_case | vl:1@5,2@5,3@5 | iv:1@5,2@5 | PRE:0@5:12,1@5:12,2@5:12 | POST:0@5:12,1@5:12,2@5:12 | PRE==POST (all backends): True
t0f_ca_if | vl:1@5,2@5,3@5 | iv:REJ | PRE:0@5:12,1@5:12,2@5:12 | POST:0@5:12,1@5:12,2@5:12 | PRE==POST (all backends): True
t0f_comb_case | vl:1@5,2@5,3@5 | iv:1@5,2@5 | PRE:1@5:12,2@5:12 | POST:1@5:12,2@5:12 | PRE==POST (all backends): True
t0f_comb_if | vl:1@5,2@5,3@5 | iv:REJ | PRE:1@5:12,2@5:12 | POST:1@5:12,2@5:12 | PRE==POST (all backends): True
q1caf_case_tbF | vl:2@16,3@16,4@16 | iv:2@16 | PRE:0@16:12,2@16:12 | POST:0@16:12,2@16:12 | PRE==POST (all backends): True
q1caf_case_tbL | vl:2@4,3@4,4@4 | iv:2@4 | PRE:0@4:12,2@4:12 | POST:0@4:12,2@4:12 | PRE==POST (all backends): True
```
t0f_ca_* = `assign y3 = f(a, b)` (function with a lone `unique if` / `unique case`), inputs written by a t0 `initial`; t0f_comb_* = same
function from `always_comb` (silent at t0, §4.5.584). q1caf_case_tbF/tbL = DUT through ports, `assign y = f(a, b)` with a `unique case`,
TB written first / last. iverilog and verilator are silent at t0 in all; PRE and POST report at time 0 (`… [in top.u] [at time 0]`).
Shape: a continuous assign whose function body holds a no-match arm, evaluated at time 0 on x before the `initial` that drives it
(a process write DOES reach it) — not 🆕 AA (no always_comb/always_latch, not a no-process-write chain). Not recorded in ROADMAP
(grep W4031 / CA t0). The candidate adds no arm a continuous assign can reach (non-void item-function bodies keep the first-`if` rule;
void functions and tasks cannot be called from a CA): q1caf/t0f PRE==POST True on all backends.

## Resume (judge ruling: CA -> function t0 report is pre-existing, recorded as its own row; not blocking)

### Q4 remainder on POST-cand (same binary md5 17b7d988696c0f5b38b7e94b845e2df9)
POSTst/PREst = staged vcmp,velab,vrun rc + (W4031/E4003+stdout == one-shot). Duplicated elab lines in POST column = one-shot + staged echo.
Q1 cells + b00 (external report repro) + s583/c b01/b02/b04/m_if_*:
```
a00_repro | vl:2@5,3@5,4@5 | iv:2@5 | PRE:2@5:12 | POST:2@5:12  | P==P:True; POSTst 000 True PREst 000 True
a08_comb_chain | vl:- | iv:5@6 | PRE:0@6:12,5@6:12 | POST:0@6:12,5@6:12  | P==P:True; POSTst 000 True PREst 000 True
a08b_comb_chain_rev | vl:- | iv:0@7 | PRE:- | POST:-  | P==P:True; POSTst 000 True PREst 000 True
b00_repro | vl:1@7,2@10 | iv:REJ | PRE:2@10:12,3@13:14 | POST:1@7:12,2@10:12,3@13:14  | P==P:False; POSTst 000 True PREst 000 True
b01_nested_qual | vl:1@5,2@7,3@9,4@11,6@15 | iv:REJ | PRE:2@7:41,3@9:43,4@11:42 | POST:1@5:15,2@7:41,3@9:43,4@11:42,6@15:15  | P==P:False; POSTst 000 True PREst 000 True
b02_frame_lanes | vl:1@7,2@13,3@18,4@22 | iv:REJ | PRE:2@13:12,4@22:12 | POST:2@13:12,3@18:12,4@22:12  | P==P:False; POSTst 000 True PREst 000 True
b03 | vl:2@6,3@6,4@6 | iv:REJ | PRE:- | POST:2@6:12,3@6:12  | P==P:False; POSTst 000 True PREst 000 True
b03H | vl:2@6,3@6,4@6 | iv:REJ | PRE:2@6:31,3@6:31 | POST:2@6:31,3@6:31  | P==P:True; POSTst 000 True PREst 000 True
b04_single_comb_t0 | vl:2@6,3@6,4@6 | iv:REJ | PRE:2@6:12,3@6:12 | POST:2@6:12,3@6:12  | P==P:True; POSTst 000 True PREst 000 True
c04_gen_intf_prog | vl:0@20,2@20,3@10,4@4 | iv:REJ | PRE:- | POST:2@20:14,2@20:14,3@10:15,4@4:12  | P==P:False; POSTst 000 True PREst 000 True
c22_tb_dut_t0 | vl:2@8,2@4,3@8,3@4,4@8,4@4 | iv:REJ | PRE:2@8:12 | POST:2@4:12,2@8:12  | P==P:False; POSTst 000 True PREst 000 True
c22b_tb_dut_t0_topfirst | vl:2@20,2@16,3@20,3@16,4@20,4@16 | iv:REJ | PRE:2@20:12 | POST:2@16:12,2@20:12  | P==P:False; POSTst 000 True PREst 000 True
c22c_case_only | vl:2@8,3@8,4@8 | iv:2@8 | PRE:2@8:12 | POST:2@8:12  | P==P:True; POSTst 000 True PREst 000 True
c23_tb_dut_chain_only | vl:2@16,3@16,4@16 | iv:REJ | PRE:- | POST:-  | P==P:True; POSTst 000 True PREst 000 True
m_if_priority | vl:- | iv:REJ | PRE:1@5:17,7@17:17 | POST:1@5:17,2@7:17,3@9:17,6@15:17,7@17:17  | P==P:False; POSTst 000 True PREst 000 True
m_if_unique | vl:1@5,2@7,3@9,6@15,7@17 | iv:REJ | PRE:1@5:15,7@17:15 | POST:1@5:15,2@7:15,3@9:15,6@15:15,7@17:15  | P==P:False; POSTst 000 True PREst 000 True
m_if_unique0 | vl:- | iv:REJ | PRE:- | POST:-  | P==P:True; POSTst 000 True PREst 000 True
q1_case_comb_tbF | vl:2@16,3@16,4@16 | iv:2@16 | PRE:2@16:12 | POST:2@16:12  | P==P:True; POSTst 000 True PREst 000 True
q1_case_comb_tbL | vl:2@4,3@4,4@4 | iv:2@4 | PRE:2@4:12 | POST:2@4:12  | P==P:True; POSTst 000 True PREst 000 True
q1_case_latch_tbF | vl:2@16,3@16,4@16 | iv:2@16 | PRE:2@16:12 | POST:2@16:12  | P==P:True; POSTst 000 True PREst 000 True
q1_case_latch_tbL | vl:2@4,3@4,4@4 | iv:2@4 | PRE:2@4:12 | POST:2@4:12  | P==P:True; POSTst 000 True PREst 000 True
q1_chain_comb_tbF | vl:2@16,3@16,4@16 | iv:REJ | PRE:- | POST:2@16:12  | P==P:False; POSTst 000 True PREst 000 True
q1_chain_comb_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:- | POST:2@4:12  | P==P:False; POSTst 000 True PREst 000 True
q1_chain_latch_tbF | vl:2@16,3@16,4@16 | iv:REJ | PRE:- | POST:2@16:12  | P==P:False; POSTst 000 True PREst 000 True
q1_chain_latch_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:- | POST:2@4:12  | P==P:False; POSTst 000 True PREst 000 True
q1_H_comb_tbF | vl:2@16,3@16,4@16 | iv:REJ | PRE:2@16:31 | POST:2@16:31  | P==P:True; POSTst 000 True PREst 000 True
q1_H_comb_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:2@4:31 | POST:2@4:31  | P==P:True; POSTst 000 True PREst 000 True
q1_H_latch_tbF | vl:2@16,3@16,4@16 | iv:REJ | PRE:2@16:31 | POST:2@16:31  | P==P:True; POSTst 000 True PREst 000 True
q1_H_latch_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:2@4:31 | POST:2@4:31  | P==P:True; POSTst 000 True PREst 000 True
q1_lone_comb_tbF | vl:2@16,3@16,4@16 | iv:REJ | PRE:2@16:12 | POST:2@16:12  | P==P:True; POSTst 000 True PREst 000 True
q1_lone_comb_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:2@4:12 | POST:2@4:12  | P==P:True; POSTst 000 True PREst 000 True
q1_lone_latch_tbF | vl:2@16,3@16,4@16 | iv:REJ | PRE:2@16:12 | POST:2@16:12  | P==P:True; POSTst 000 True PREst 000 True
q1_lone_latch_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:2@4:12 | POST:2@4:12  | P==P:True; POSTst 000 True PREst 000 True
q1ca_case_tbF | vl:2@17,3@17,4@17 | iv:2@17 | PRE:2@17:12 | POST:2@17:12  | P==P:True; POSTst 000 True PREst 000 True
q1ca_case_tbL | vl:2@4,3@4,4@4 | iv:2@4 | PRE:2@4:12 | POST:2@4:12  | P==P:True; POSTst 000 True PREst 000 True
q1ca_chain_tbF | vl:2@17,3@17,4@17 | iv:REJ | PRE:- | POST:2@17:12  | P==P:False; POSTst 000 True PREst 000 True
q1ca_chain_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:- | POST:2@4:12  | P==P:False; POSTst 000 True PREst 000 True
q1ca_H_tbF | vl:2@17,3@17,4@17 | iv:REJ | PRE:2@17:31 | POST:2@17:31  | P==P:True; POSTst 000 True PREst 000 True
q1ca_H_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:2@4:31 | POST:2@4:31  | P==P:True; POSTst 000 True PREst 000 True
q1ca_lone_tbF | vl:2@17,3@17,4@17 | iv:REJ | PRE:2@17:12 | POST:2@17:12  | P==P:True; POSTst 000 True PREst 000 True
q1ca_lone_tbL | vl:2@4,3@4,4@4 | iv:REJ | PRE:2@4:12 | POST:2@4:12  | P==P:True; POSTst 000 True PREst 000 True
q1caf_case_tbF | vl:2@16,3@16,4@16 | iv:2@16 | PRE:0@16:12,2@16:12 | POST:0@16:12,2@16:12  | P==P:True; POSTst 000 True PREst 000 True
q1caf_case_tbL | vl:2@4,3@4,4@4 | iv:2@4 | PRE:0@4:12,2@4:12 | POST:0@4:12,2@4:12  | P==P:True; POSTst 000 True PREst 000 True
q1d_case_leafF | vl:2@4,3@4,4@4 | iv:2@4 | PRE:2@4:12 | POST:2@4:12  | P==P:True; POSTst 000 True PREst 000 True
q1d_case_tbF | vl:2@21,3@21,4@21 | iv:2@21 | PRE:2@21:12 | POST:2@21:12  | P==P:True; POSTst 000 True PREst 000 True
q1d_chain_leafF | vl:2@4,3@4,4@4 | iv:REJ | PRE:- | POST:2@4:12  | P==P:False; POSTst 000 True PREst 000 True
q1d_chain_tbF | vl:2@21,3@21,4@21 | iv:REJ | PRE:- | POST:2@21:12  | P==P:False; POSTst 000 True PREst 000 True
q1d_H_leafF | vl:2@4,3@4,4@4 | iv:REJ | PRE:2@4:31 | POST:2@4:31  | P==P:True; POSTst 000 True PREst 000 True
q1d_H_tbF | vl:2@21,3@21,4@21 | iv:REJ | PRE:2@21:31 | POST:2@21:31  | P==P:True; POSTst 000 True PREst 000 True
q1d_lone_leafF | vl:2@4,3@4,4@4 | iv:REJ | PRE:2@4:12 | POST:2@4:12  | P==P:True; POSTst 000 True PREst 000 True
q1d_lone_tbF | vl:2@21,3@21,4@21 | iv:REJ | PRE:2@21:12 | POST:2@21:12  | P==P:True; POSTst 000 True PREst 000 True
t0f_both_case | vl:1@5,2@5,3@5 | iv:1@5,1@5,2@5 | PRE:0@5:12,1@5:12,1@5:12,2@5:12 | POST:0@5:12,1@5:12,1@5:12,2@5:12  | P==P:True; POSTst 000 True PREst 000 True
t0f_ca_case | vl:1@5,2@5,3@5 | iv:1@5,2@5 | PRE:0@5:12,1@5:12,2@5:12 | POST:0@5:12,1@5:12,2@5:12  | P==P:True; POSTst 000 True PREst 000 True
t0f_ca_if | vl:1@5,2@5,3@5 | iv:REJ | PRE:0@5:12,1@5:12,2@5:12 | POST:0@5:12,1@5:12,2@5:12  | P==P:True; POSTst 000 True PREst 000 True
t0f_comb_case | vl:1@5,2@5,3@5 | iv:1@5,2@5 | PRE:1@5:12,2@5:12 | POST:1@5:12,2@5:12  | P==P:True; POSTst 000 True PREst 000 True
t0f_comb_if | vl:1@5,2@5,3@5 | iv:REJ | PRE:1@5:12,2@5:12 | POST:1@5:12,2@5:12  | P==P:True; POSTst 000 True PREst 000 True
```
Q3 cells (PRE-H = H spelling on PRE):
```
cls_ctor_rt | vl:1@5 | PRE:- | PRE-H:1@5:36 | POST:1@5:12  | P==P:False; POSTst 000 True PREst 000 True
cls_fn_rt | vl:1@4 | PRE:- | PRE-H:1@4:36 | POST:1@4:12  | P==P:False; POSTst 000 True PREst 000 True
cls_sfn_k | vl:- | PRE:elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:31(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002) rc=1 | PRE-H:elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:24(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002) rc=1 | POST:elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:31(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002),elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:31(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002),elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:31(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002) rc=1  | P==P:True; POSTst 133 False PREst 133 False
cls_sfn_rt | vl:1@4 | PRE:elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:31(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002) rc=1 | PRE-H:elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:24(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002) rc=1 | POST:elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:31(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002),elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:31(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002),elab@2:3(VITA-E2002),elab@3:12(VITA-E2002),elab@4:5(VITA-E2002),elab@4:31(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002) rc=1  | P==P:True; POSTst 133 False PREst 133 False
cls_task_rt | vl:1@5 | PRE:- | PRE-H:1@5:36 | POST:1@5:12  | P==P:False; POSTst 000 True PREst 000 True
cls_vfn_rt | vl:1@5 | PRE:- | PRE-H:1@5:36 | POST:1@5:12  | P==P:False; POSTst 000 True PREst 000 True
cls_vfn_via_f_rt | vl:1@4 | PRE:elab@6:32(VITA-E3010) rc=1 | PRE-H:elab@6:32(VITA-E3010) rc=1 | POST:elab@6:32(VITA-E3010),elab@6:32(VITA-E3010),elab@6:32(VITA-E3010) rc=1  | P==P:True; POSTst 013 False PREst 013 False
gen_fn_k | vl:COMPILE-FAIL | PRE:elab@8:24(VITA-E3009),elab@9:13(VITA-E3010) rc=1 | PRE-H:elab@8:24(VITA-E3009),elab@9:13(VITA-E3010) rc=1 | POST:elab@8:24(VITA-E3009),elab@9:13(VITA-E3010),elab@8:24(VITA-E3009),elab@9:13(VITA-E3010),elab@8:24(VITA-E3009),elab@9:13(VITA-E3010) rc=1  | P==P:True; POSTst 013 False PREst 013 False
gen_fn_rt | vl:1@5 | PRE:- | PRE-H:1@5:38 | POST:-  | P==P:True; POSTst 000 True PREst 000 True
ifc_fn_k | vl:- | PRE:- | PRE-H:elab@7:22(VITA-E3009) rc=1 | POST:-  | P==P:True; POSTst 000 True PREst 000 True
ifc_fn_rt | vl:1@4 | PRE:- | PRE-H:1@4:36 | POST:-  | P==P:True; POSTst 000 True PREst 000 True
ifc_vfn_rt | vl:1@4 | PRE:- | PRE-H:1@4:36 | POST:1@4:12  | P==P:False; POSTst 000 True PREst 000 True
m_auto_k | vl:- | PRE:- | PRE-H:elab@7:22(VITA-E3009) rc=1 | POST:-  | P==P:True; POSTst 000 True PREst 000 True
m_auto_rt | vl:1@4 | PRE:- | PRE-H:1@4:36 | POST:-  | P==P:True; POSTst 000 True PREst 000 True
m_static_k | vl:- | PRE:- | PRE-H:elab@7:22(VITA-E3009) rc=1 | POST:-  | P==P:True; POSTst 000 True PREst 000 True
m_static_kgen | vl:- | PRE:- | PRE-H:elab@7:7(VITA-E3010) rc=1 | POST:-  | P==P:True; POSTst 000 True PREst 000 True
m_static_khdr | vl:- | PRE:- | PRE-H:elab@1:32(VITA-E3009) rc=1 | POST:-  | P==P:True; POSTst 000 True PREst 000 True
m_static_rt | vl:1@4 | PRE:- | PRE-H:1@4:36 | POST:-  | P==P:True; POSTst 000 True PREst 000 True
m_vfn_via_f_rt | vl:1@4 | PRE:elab@1:8(VITA-E3009) rc=1 | PRE-H:elab@1:8(VITA-E3009) rc=1 | POST:elab@1:8(VITA-E3009),elab@1:8(VITA-E3009),elab@1:8(VITA-E3009) rc=1  | P==P:True; POSTst 013 False PREst 013 False
m_vfn2_via_f_k | vl:COMPILE-FAIL | PRE:elab@7:22(VITA-E3009),elab@1:8(VITA-E3009) rc=1 | PRE-H:elab@7:22(VITA-E3009),elab@1:8(VITA-E3009) rc=1 | POST:elab@7:22(VITA-E3009),elab@1:8(VITA-E3009),elab@7:22(VITA-E3009),elab@1:8(VITA-E3009),elab@7:22(VITA-E3009),elab@1:8(VITA-E3009) rc=1  | P==P:True; POSTst 013 False PREst 013 False
m_vfn2_via_f_rt | vl:1@4 | PRE:elab@1:8(VITA-E3009) rc=1 | PRE-H:elab@1:8(VITA-E3009) rc=1 | POST:elab@1:8(VITA-E3009),elab@1:8(VITA-E3009),elab@1:8(VITA-E3009) rc=1  | P==P:True; POSTst 013 False PREst 013 False
m_void_k | vl:COMPILE-FAIL | PRE:elab@7:22(VITA-E3009),elab@1:8(VITA-E3009) rc=1 | PRE-H:elab@7:22(VITA-E3009),elab@1:8(VITA-E3009) rc=1 | POST:elab@7:22(VITA-E3009),elab@1:8(VITA-E3009),elab@7:22(VITA-E3009),elab@1:8(VITA-E3009),elab@7:22(VITA-E3009),elab@1:8(VITA-E3009) rc=1  | P==P:True; POSTst 013 False PREst 013 False
m_void_rt | vl:1@4 | PRE:- | PRE-H:1@4:36 | POST:1@4:12  | P==P:False; POSTst 000 True PREst 000 True
pkg_fn_k | vl:- | PRE:- | PRE-H:elab@9:22(VITA-E3009) rc=1 | POST:-  | P==P:True; POSTst 000 True PREst 000 True
pkg_fn_kimp | vl:- | PRE:- | PRE-H:elab@10:22(VITA-E3009) rc=1 | POST:-  | P==P:True; POSTst 000 True PREst 000 True
pkg_fn_rt | vl:1@4 | PRE:- | PRE-H:elab@9:20(VITA-E3009) rc=1 | POST:-  | P==P:True; POSTst 000 True PREst 000 True
pkg_task_imp_rt | vl:1@8 | PRE:elab@14:8(VITA-E3009) rc=1 | PRE-H:elab@14:8(VITA-E3009) rc=1 | POST:elab@14:8(VITA-E3009),elab@14:8(VITA-E3009),elab@14:8(VITA-E3009) rc=1  | P==P:True; POSTst 013 False PREst 013 False
pkg_vfn_imp_rt | vl:1@4 | PRE:elab@14:8(VITA-E3009) rc=1 | PRE-H:elab@14:8(VITA-E3009) rc=1 | POST:elab@14:8(VITA-E3009),elab@14:8(VITA-E3009),elab@14:8(VITA-E3009) rc=1  | P==P:True; POSTst 013 False PREst 013 False
pkg_vfn_via_f_imp_rt | vl:1@4 | PRE:elab@14:8(VITA-E3009) rc=1 | PRE-H:elab@14:8(VITA-E3009) rc=1 | POST:elab@14:8(VITA-E3009),elab@14:8(VITA-E3009),elab@14:8(VITA-E3009) rc=1  | P==P:True; POSTst 013 False PREst 013 False
pkg_vfn_via_f_k | vl:COMPILE-FAIL | PRE:elab@15:22(VITA-E3009) rc=1 | PRE-H:elab@15:22(VITA-E3009) rc=1 | POST:elab@15:22(VITA-E3009),elab@15:22(VITA-E3009),elab@15:22(VITA-E3009) rc=1  | P==P:True; POSTst 013 False PREst 013 False
pkg_vfn_via_f_rt | vl:1@4 | PRE:elab@15:20(VITA-E3009) rc=1 | PRE-H:elab@15:20(VITA-E3009) rc=1 | POST:elab@15:20(VITA-E3009),elab@15:20(VITA-E3009),elab@15:20(VITA-E3009) rc=1  | P==P:False; POSTst 013 False PREst 013 False
pkg_vfn2_imp_rt | vl:1@4 | PRE:elab@10:8(VITA-E3009) rc=1 | PRE-H:elab@10:8(VITA-E3009) rc=1 | POST:elab@10:8(VITA-E3009),elab@10:8(VITA-E3009),elab@10:8(VITA-E3009) rc=1  | P==P:True; POSTst 013 False PREst 013 False
pkg_vfn2_via_f_imp_rt | vl:1@4 | PRE:elab@10:8(VITA-E3009) rc=1 | PRE-H:elab@10:8(VITA-E3009) rc=1 | POST:elab@10:8(VITA-E3009),elab@10:8(VITA-E3009),elab@10:8(VITA-E3009) rc=1  | P==P:True; POSTst 013 False PREst 013 False
pkg_vfn2_via_f_rt | vl:1@4 | PRE:elab@11:20(VITA-E3009) rc=1 | PRE-H:elab@11:20(VITA-E3009) rc=1 | POST:elab@11:20(VITA-E3009),elab@11:20(VITA-E3009),elab@11:20(VITA-E3009) rc=1  | P==P:False; POSTst 013 False PREst 013 False
prog_fn_rt | vl:1@4 | PRE:- | PRE-H:1@4:36 | POST:-  | P==P:True; POSTst 000 True PREst 000 True
unit_fn_k | vl:- | PRE:- | PRE-H:elab@7:22(VITA-E3009) rc=1 | POST:-  | P==P:True; POSTst 000 True PREst 000 True
unit_fn_rt | vl:1@3 | PRE:- | PRE-H:1@3:34 | POST:-  | P==P:True; POSTst 000 True PREst 000 True
pkg_vfn3_imp_rt | vl:1@4,3@8 | PRE:- | PRE-H:1@4:36,3@8:36 | POST:1@4:12,3@8:12  | P==P:False; POSTst 000 True PREst 000 True
```
pkg_vfn_via_f_rt / pkg_vfn2_via_f_rt: P==P False = E3009 text change only (rc=1 both): PRE `frame function/task pk::f body uses an assignment to a
net outside the function …`; POST `package-scoped call pk::f(...) reaches pk::fv, whose body names something outside its own formals/locals …`.
Remaining S/s583/c cells (case-only, no if chains):
```
a00p_repro_initfirst_pure | vl:2@11,3@11,4@11 | iv:2@11 | PRE:2@11:12 | POST:2@11:12  | P==P:True; POSTst 000 True PREst 000 True
a00s_repro_initfirst | vl:2@13,3@13,4@13 | iv:2@13 | PRE:2@13:12 | POST:2@13:12  | P==P:True; POSTst 000 True PREst 000 True
a00t_repro_trace | vl:2@6,3@6,4@6 | iv:2@6 | PRE:2@6:12 | POST:2@6:12  | P==P:True; POSTst 000 True PREst 000 True
a01_comb_hash0_glitch | vl:5@6 | iv:5@6 | PRE:5@6:12 | POST:5@6:12  | P==P:True; POSTst 000 True PREst 000 True
a02_star_hash0_glitch | vl:5@6 | iv:5@6 | PRE:5@6:12 | POST:5@6:12  | P==P:True; POSTst 000 True PREst 000 True
a03_atr_hash0_glitch | vl:5@6 | iv:5@6 | PRE:5@6:12 | POST:5@6:12  | P==P:True; POSTst 000 True PREst 000 True
a04_latch_hash0_glitch | vl:5@6 | iv:5@6 | PRE:5@6:12 | POST:5@6:12  | P==P:True; POSTst 000 True PREst 000 True
a05_posedge_double | vl:5@7 | iv:5@7 | PRE:5@7:12 | POST:5@7:12  | P==P:True; POSTst 000 True PREst 000 True
a06_initial_two_writes | vl:- | iv:- | PRE:- | POST:-  | P==P:True; POSTst 000 True PREst 000 True
a07_nba_chain | vl:5@9 | iv:5@9 | PRE:5@9:12 | POST:5@9:12  | P==P:True; POSTst 000 True PREst 000 True
a09_comb_persist | vl:5@6,6@6,7@6 | iv:5@6 | PRE:5@6:12 | POST:5@6:12  | P==P:True; POSTst 000 True PREst 000 True
a10_resume_event | vl:5@8 | iv:5@8 | PRE:5@8:12 | POST:5@8:12  | P==P:True; POSTst 000 True PREst 000 True
a10w_resume_wait | vl:5@8 | iv:5@8 | PRE:5@8:12 | POST:5@8:12  | P==P:True; POSTst 000 True PREst 000 True
a11_same_proc_hash0 | vl:5@7 | iv:5@7 | PRE:5@7:12 | POST:5@7:12  | P==P:True; POSTst 000 True PREst 000 True
a12_func_two_procs | vl:5@5 | iv:5@5,5@5 | PRE:5@5:12,5@5:12 | POST:5@5:12,5@5:12  | P==P:True; POSTst 000 True PREst 000 True
a13_initial_x_t0 | vl:0@5 | iv:0@5 | PRE:0@5:12 | POST:0@5:12  | P==P:True; POSTst 000 True PREst 000 True
a14_comb_t0_nba_init | vl:- | iv:0@6 | PRE:0@6:12 | POST:0@6:12  | P==P:True; POSTst 000 True PREst 000 True
a15_comb_t0_hash0_init | vl:0@6 | iv:- | PRE:0@6:12 | POST:0@6:12  | P==P:True; POSTst 000 True PREst 000 True
f01_finish_same_step | vl:5@6 | iv:- | PRE:5@6:12 | POST:5@6:12  | P==P:True; POSTst 000 True PREst 000 True
f02_finish_after_hash0 | vl:5@6 | iv:5@6 | PRE:5@6:12 | POST:5@6:12  | P==P:True; POSTst 000 True PREst 000 True
m_case_priority | vl:1@5,2@10,3@15 | iv:1@5,2@10,3@15 | PRE:1@5:17,2@10:17,3@15:17 | POST:1@5:17,2@10:17,3@15:17  | P==P:True; POSTst 000 True PREst 000 True
m_case_unique | vl:1@5,2@10,3@15 | iv:1@5,2@10,3@15 | PRE:1@5:15,2@10:15,3@15:15 | POST:1@5:15,2@10:15,3@15:15  | P==P:True; POSTst 000 True PREst 000 True
m_case_unique0 | vl:- | iv:- | PRE:- | POST:-  | P==P:True; POSTst 000 True PREst 000 True
o01_t0_hash0hash0 | vl:0@6 | iv:0@6 | PRE:0@6:12 | POST:0@6:12  | P==P:True; POSTst 000 True PREst 000 True
o02_t0_hash0x3 | vl:0@6 | iv:0@6 | PRE:0@6:12 | POST:0@6:12  | P==P:True; POSTst 000 True PREst 000 True
o03_t0_hash0_then_nba | vl:0@6 | iv:0@6 | PRE:0@6:12 | POST:0@6:12  | P==P:True; POSTst 000 True PREst 000 True
o04_t0_always_hash0 | vl:0@6 | iv:- | PRE:0@6:12 | POST:0@6:12  | P==P:True; POSTst 000 True PREst 000 True
o05_t0_assign | vl:- | iv:- | PRE:- | POST:-  | P==P:True; POSTst 000 True PREst 000 True
o06_t0_initial_display | vl:0@6 | iv:0@6 | PRE:0@6:12 | POST:0@6:12  | P==P:True; POSTst 000 True PREst 000 True
o07_three_combs | vl:- | iv:- | PRE:- | POST:-  | P==P:True; POSTst 000 True PREst 000 True
r01_order | vl:5@7,7@7 | iv:5@7 | PRE:5@7:12 | POST:5@7:12  | P==P:True; POSTst 000 True PREst 000 True
v01_comb_const_t0_read | vl:- | iv:- | PRE:- | POST:-  | P==P:True; POSTst 000 True PREst 000 True
v02_comb_const_initial_first | vl:- | iv:- | PRE:- | POST:-  | P==P:True; POSTst 000 True PREst 000 True
v03_latch_t0 | vl:- | iv:- | PRE:- | POST:-  | P==P:True; POSTst 000 True PREst 000 True
v04_selftimed_always_order | vl:- | iv:- | PRE:- | POST:-  | P==P:True; POSTst 000 True PREst 000 True
```

### Item 2 — new-arm reachability from a CA at t0 (cells S/s585/g/c7/n_*.sv)
Stimulus: t0 a=0 b=1 (match), t2 b=0 (no match). tbF / tbL = TB module written first / last.
```
n_ca_Csf_tbF | vl:2@16,3@16,4@16 | iv:REJ | s2v:ran | PRE:elab@14:3(VITA-E2002),elab@15:20(VITA-E2002),elab@16:5(VITA-E2002),elab@16:26(VITA-E2002),elab@17:5(VITA-E2002),elab@18:3(VITA-E2002) rc=1 | POST:elab@14:3(VITA-E2002),elab@15:20(VITA-E2002),elab@16:5(VITA-E2002),elab@16:26(VITA-E2002),elab@17:5(VITA-E2002),elab@18:3(VITA-E2002),elab@14:3(VITA-E2002),elab@15:20(VITA-E2002),elab@16:5(VITA-E2002),elab@16:26(VITA-E2002),elab@17:5(VITA-E2002),elab@18:3(VITA-E2002),elab@14:3(VITA-E2002),elab@15:20(VITA-E2002),elab@16:5(VITA-E2002),elab@16:26(VITA-E2002),elab@17:5(VITA-E2002),elab@18:3(VITA-E2002) rc=1 | P==P:True; POSTst 133 False PREst 133 False
n_ca_Csf_tbL | vl:2@4,3@4,4@4 | iv:REJ | s2v:ran | PRE:elab@2:3(VITA-E2002),elab@3:20(VITA-E2002),elab@4:5(VITA-E2002),elab@4:26(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002) rc=1 | POST:elab@2:3(VITA-E2002),elab@3:20(VITA-E2002),elab@4:5(VITA-E2002),elab@4:26(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002),elab@2:3(VITA-E2002),elab@3:20(VITA-E2002),elab@4:5(VITA-E2002),elab@4:26(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002),elab@2:3(VITA-E2002),elab@3:20(VITA-E2002),elab@4:5(VITA-E2002),elab@4:26(VITA-E2002),elab@5:5(VITA-E2002),elab@6:3(VITA-E2002) rc=1 | P==P:True; POSTst 133 False PREst 133 False
n_ca_ifmp_tbF | vl:2@16,3@16,4@16 | iv:REJ | s2v:FAIL | PRE:elab@19:14(VITA-E2002),elab@19:14(VITA-E2002),elab@19:14(VITA-E2002),elab@19:22(VITA-E2002) rc=1 | POST:elab@19:14(VITA-E2002),elab@19:14(VITA-E2002),elab@19:14(VITA-E2002),elab@19:22(VITA-E2002),elab@19:14(VITA-E2002),elab@19:14(VITA-E2002),elab@19:14(VITA-E2002),elab@19:22(VITA-E2002),elab@19:14(VITA-E2002),elab@19:14(VITA-E2002),elab@19:14(VITA-E2002),elab@19:22(VITA-E2002) rc=1 | P==P:True; POSTst 133 False PREst 133 False
n_ca_ifmp_tbL | vl:2@4,3@4,4@4 | iv:REJ | s2v:FAIL | PRE:elab@7:14(VITA-E2002),elab@7:14(VITA-E2002),elab@7:14(VITA-E2002),elab@7:22(VITA-E2002) rc=1 | POST:elab@7:14(VITA-E2002),elab@7:14(VITA-E2002),elab@7:14(VITA-E2002),elab@7:22(VITA-E2002),elab@7:14(VITA-E2002),elab@7:14(VITA-E2002),elab@7:14(VITA-E2002),elab@7:22(VITA-E2002),elab@7:14(VITA-E2002),elab@7:14(VITA-E2002),elab@7:14(VITA-E2002),elab@7:22(VITA-E2002) rc=1 | P==P:True; POSTst 133 False PREst 133 False
n_ca_ifport_tbF | vl:2@16,3@16,4@16 | iv:REJ | s2v:ran | PRE:elab@21:14(VITA-E3009) rc=1 | POST:elab@21:14(VITA-E3009),elab@21:14(VITA-E3009),elab@21:14(VITA-E3009) rc=1 | P==P:True; POSTst 013 False PREst 013 False
n_ca_ifport_tbL | vl:2@4,3@4,4@4 | iv:REJ | s2v:ran | PRE:elab@9:14(VITA-E3009) rc=1 | POST:elab@9:14(VITA-E3009),elab@9:14(VITA-E3009),elab@9:14(VITA-E3009) rc=1 | P==P:True; POSTst 013 False PREst 013 False
n_ca_mf_tbF | vl:2@16,3@16,4@16 | iv:REJ | s2v:ran | PRE:- | POST:- | P==P:True; POSTst 000 True PREst 000 True
n_ca_mf_tbL | vl:2@4,3@4,4@4 | iv:REJ | s2v:ran | PRE:- | POST:- | P==P:True; POSTst 000 True PREst 000 True
n_ca_objf_tbF | vl:2@16,3@16,4@16 | iv:REJ | s2v:FAIL | PRE:elab@21:3(VITA-E3009),elab@3:7(VITA-E3010),elab@3:7(VITA-E3009),elab@22:14(VITA-E3009) rc=1 | POST:elab@21:3(VITA-E3009),elab@3:7(VITA-E3010),elab@3:7(VITA-E3009),elab@22:14(VITA-E3009),elab@21:3(VITA-E3009),elab@3:7(VITA-E3010),elab@3:7(VITA-E3009),elab@22:14(VITA-E3009),elab@21:3(VITA-E3009),elab@3:7(VITA-E3010),elab@3:7(VITA-E3009),elab@22:14(VITA-E3009) rc=1 | P==P:True; POSTst 013 False PREst 013 False
n_ca_objf_tbL | vl:2@4,3@4,4@4 | iv:REJ | s2v:FAIL | PRE:elab@9:3(VITA-E3009),elab@14:7(VITA-E3010),elab@14:7(VITA-E3009),elab@10:14(VITA-E3009) rc=1 | POST:elab@9:3(VITA-E3009),elab@14:7(VITA-E3010),elab@14:7(VITA-E3009),elab@10:14(VITA-E3009),elab@9:3(VITA-E3009),elab@14:7(VITA-E3010),elab@14:7(VITA-E3009),elab@10:14(VITA-E3009),elab@9:3(VITA-E3009),elab@14:7(VITA-E3010),elab@14:7(VITA-E3009),elab@10:14(VITA-E3009) rc=1 | P==P:True; POSTst 013 False PREst 013 False
n_ca_objf2_tbF | vl:2@16,3@16,4@16 | PRE:- | POST:0@16:12,0@16:12,0@16:12,0@16:12,2@16:12,2@16:12,2@16:12,3@16:12,3@16:12,4@16:12,4@16:12 | P==P:False; POSTst 000 True PREst 000 True
n_ca_objf2_tbL | vl:2@4,3@4,4@4 | iv:REJ | s2v:FAIL | PRE:- | PRE-H:0@4:31,0@4:31,0@4:31,0@4:31,2@4:31,2@4:31,2@4:31,3@4:31,3@4:31,4@4:31,4@4:31 | POST:0@4:12,0@4:12,0@4:12,0@4:12,2@4:12,2@4:12,2@4:12,3@4:12,3@4:12,4@4:12,4@4:12 | P==P:False
n_ca_objf2case_tbL | vl:2@4,3@4,4@4 | iv:- | s2v:FAIL | PRE:0@4:12,0@4:12,0@4:12,0@4:12,2@4:12,2@4:12,2@4:12,3@4:12,3@4:12,4@4:12,4@4:12 | POST:0@4:12,0@4:12,0@4:12,0@4:12,2@4:12,2@4:12,2@4:12,3@4:12,3@4:12,4@4:12,4@4:12 | P==P:True
n_comb_objf_tbF | vl:2@16,3@16,4@16 | iv:REJ | s2v:FAIL | PRE:elab@21:3(VITA-E3009),elab@3:7(VITA-E3010),elab@3:7(VITA-E3009),elab@22:19(VITA-E3009) rc=1 | POST:elab@21:3(VITA-E3009),elab@3:7(VITA-E3010),elab@3:7(VITA-E3009),elab@22:19(VITA-E3009),elab@21:3(VITA-E3009),elab@3:7(VITA-E3010),elab@3:7(VITA-E3009),elab@22:19(VITA-E3009),elab@21:3(VITA-E3009),elab@3:7(VITA-E3010),elab@3:7(VITA-E3009),elab@22:19(VITA-E3009) rc=1 | P==P:True; POSTst 013 False PREst 013 False
n_comb_objf_tbL | vl:2@4,3@4,4@4 | iv:REJ | s2v:FAIL | PRE:elab@9:3(VITA-E3009),elab@14:7(VITA-E3010),elab@14:7(VITA-E3009),elab@10:19(VITA-E3009) rc=1 | POST:elab@9:3(VITA-E3009),elab@14:7(VITA-E3010),elab@14:7(VITA-E3009),elab@10:19(VITA-E3009),elab@9:3(VITA-E3009),elab@14:7(VITA-E3010),elab@14:7(VITA-E3009),elab@10:19(VITA-E3009),elab@9:3(VITA-E3009),elab@14:7(VITA-E3010),elab@14:7(VITA-E3009),elab@10:19(VITA-E3009) rc=1 | P==P:True; POSTst 013 False PREst 013 False
n_comb_objf2_tbF | vl:2@16,3@16,4@16 | PRE:- | POST:2@16:12 | P==P:False; POSTst 000 True PREst 000 True
n_comb_objf2_tbL | vl:2@4,3@4,4@4 | PRE:- | POST:2@4:12 | P==P:False; POSTst 000 True PREst 000 True
n_comb_task_tbF | vl:2@16,3@16,4@16 | iv:REJ | s2v:ran | PRE:- | POST:2@16:12 | P==P:False; POSTst 000 True PREst 000 True
n_comb_task_tbL | vl:2@4,3@4,4@4 | iv:REJ | s2v:ran | PRE:- | POST:2@4:12 | P==P:False; POSTst 000 True PREst 000 True
n_comb_vfn_one | vl:2@5,3@5,4@5 | iv:REJ | s2v:ran | PRE:- | POST:2@5:12 | P==P:False; POSTst 000 True PREst 000 True
n_comb_vfn_tbF | vl:2@16,3@16,4@16 | iv:REJ | s2v:ran | PRE:- | POST:2@16:12 | P==P:False; POSTst 000 True PREst 000 True
n_comb_vfn_tbL | vl:2@4,3@4,4@4 | iv:REJ | s2v:ran | PRE:- | POST:2@4:12 | P==P:False; POSTst 000 True PREst 000 True
n_latch_vfn_tbF | vl:2@16,3@16,4@16 | iv:REJ | s2v:ran | PRE:- | POST:2@16:12 | P==P:False; POSTst 000 True PREst 000 True
n_latch_vfn_tbL | vl:2@4,3@4,4@4 | iv:REJ | s2v:ran | PRE:- | POST:2@4:12 | P==P:False; POSTst 000 True PREst 000 True```
Refusals (PRE = POST): n_ca_objf (`C obj = new;` decl init): `E3009 … a class-handle declaration initializer is outside the N7 MVP` +
`E3009 … unsupported hierarchical function call obj.f`; n_ca_Csf: `E2002 … N7 MVP does not support rand/randc/static/… class members, found
identifier 'static'`; n_ca_ifport: `E3009 … unsupported hierarchical function call p.f (the callee must be a framed function …`; n_ca_ifmp:
`E2002 … expected a direction (input/output/inout) before the first modport member, found keyword 'import'`.
n_ca_mf (module non-void fn in a CA): PRE = POST silent (unarmed). Void fn / task / class method from always_comb or always_latch: POST reports
at t2 only = verilator (no t0).

STOP — REGRESSION: n_ca_objf2_tbF / n_ca_objf2_tbL (`C obj; initial obj = new;` + `assign y = obj.f(a, b);`, chain in a class non-void function):
- verilator: `t=1 y=2`, `[2] %Error: n_ca_objf2_tbL.sv:4: Assertion failed in $unit.C.f: 'unique if' statement violated` (x2 per step at t2, t3, t4); nothing at t0.
- iverilog: compile rc=4 (`n_ca_objf2_tbL.sv:4: syntax error` — rejects `unique if`); unique-case analog n_ca_objf2case_tbL: run rc=-6
  `internal error: 14vvp_wide_fun_t: recv_object(...) not implemented`. sv2v: `Parse error: unexpected token 'new' (KW_new)`.
- PRE (native=interp=vm): silent (`t=1 y=2`, `t=3 y=0`).
- POST (native=interp=vm, staged identical): `n_ca_objf2_tbL.sv:4:12: warning[VITA-W4031] W-RUN-UNIQUE-VIOLATION: value is unhandled for priority
  or unique case statement [in top.C.f] [at time 0]` x4, then x3 at t2, x2 at t3, x2 at t4.
- PRE-H (armed tree on PRE) and the unique-case analog on PRE: the same 4 x t0 reports -> root = the pre-existing CA t0 row; the candidate's
  new arm on class non-void functions makes it reachable from `if` chains.
- Fix option inside the flag (not built, not measured): `in_const_fn_body = !is_void` also for class methods (class non-void functions keep the
  first-if rule; class void methods and tasks cannot be called from a CA). Cost: chains in class non-void functions stay silent where verilator
  reports at run time (cls_fn_rt `$unit.C.f`, snd_c3p line 8 `top.C.f`); a class `function new` parses with is_void = false, so the ctor
  (cls_ctor_rt, dif_c21 line 12) would also go silent unless excluded.
Items 3 (Q5 JIT / --no-default-features) and 4 (attempt tests) NOT started.
