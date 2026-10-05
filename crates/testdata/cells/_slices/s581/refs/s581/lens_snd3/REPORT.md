# lens:soundness round 3

status: started

## mutants launched (a: fallback removed, b: .max(pw) dropped), logs /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s581/lens_snd3/mut_{a,b}.log

## probes written: p/k.sv (C*), p/f.sv (F*)

## Q1 result (probes p/k.sv, p/f.sv, p/f2.sv; raw in p/out/)
- RIGHT->LOUD on POST3b (PRE = iverilog = sv2v): C02 C03 C11 C14 C15 -- lhs with unknown const_self_width
  (replication with a parameter count `{N{2'b10}}`, also inside ?: and {}), PRE answers via
  const_int_selfdet (w=0 degrade) in the masked compare; POST3b `const_self_width(lhs)?` declines, no fallback.
- moved and = oracles: C08 (0->1) C28 (0->1) C18 C25 C26 C27 C29 C30 (loud->value)
- function lane F1..F13: E3009 on PRE and POST3b, with and without an arg env (oracles answer) -> = PRE

## Q1b consumers of the unknown-width class (p/g.sv)
- G1 generate-case label `({N{2'b10}} ==? 4'b1?10)`: PRE arm a, POST3b ARM=def rc0 (iv a, sv2v a)  -> RIGHT->SILENT-WRONG
- R2 range bound `[({N{2'b10}} ==? 4'b1?10) + 2 : 0]`: PRE $bits 4, POST3b $bits 1 rc0 (iv 4, sv2v 4) -> RIGHT->SILENT-WRONG
- G2 gen-case scrutinee, R1 `(P & {W{1'b1}}) ==? ...`, R5 ternary cond in typed localparam: RIGHT->LOUD
- G3 (!=? label, answer def) R6 R7: = PRE

## Q1c attribution (p/b.sv)
- B1 `[("AB" ==? 16'h4?42)+2:0]` and B2 `[({N{2'b10}} inside {4'b1?10})+2:0]`: PRE $bits 1, POST3b $bits 1 (oracles 4)
  => a DECLINED bound silently collapses to 1 bit on PRE too (pre-existing consumer weakness, AD_X/RB_X family);
  R2 is new because POST3b declines a bound PRE folded (PRE 4 = oracles).
- B3 / G1L controls (literal count `{2{2'b10}}`): PRE = POST3b = oracles (4 / arm a) => trigger is the unknown
  const_self_width of the replication with a parameter count.

## Census (task 1)
- const_wildcard_i64: 1 caller, const_fn_width.rs:559 (width-aware comparison arm, under env.is_empty() && envw.is_empty(), after const_compare_special)
- const_wildcard_masked_pre: 1 caller, wildcard_eq.rs:218, inside `if w > 64` only
- const_compare_special: 1 caller on PRE and POST3b, const_fn_width.rs:552 (same arm) => old masked compare's reach == new routine's reach
- other i64 WildEq/WildNe/InsideEq sites: const_fn.rs:131/134 (x/z-free collapse to ==/!=), const_eval.rs:1527 & const_real.rs:194 (InsideEq as ==), const_fn_width.rs:105-107 (classification), const_bound.rs:752 (diagnostic text only), const_level_header.rs:574-577 (names)
- fallback vs PRE const_str.rs (e54fa74a) masked compare: same predicate set (WildEq/WildNe, rhs IntLit Sized un-stripped, const_int_selfdet(lhs), unk!=0, a>=0, 1 word, bit63 clear) - check order only differs; equivalent
- declines of const_wildcard_i64 and PRE on the same input:
  op gate / non-IntLit pattern / fill literal / parse fail / no x/z bit : PRE declined too (= PRE)
  w > 64 : fallback (= PRE)
  const_self_width(lhs) None : PRE ANSWERED (const_int_selfdet degrades to w=0) -> FINDING S3-1
  eval_const_env_at(lhs,w,sg) None : 7 probes (C05 C06 C07 C09 C10 C16 C20) all = PRE; not exhaustive

## mutants
== mut_a rc=100 wall=353s
FAIL [   0.016s] cli::inside_wildcard a_wide_declaration_with_a_fitting_value_keeps_the_pre_compare
     Summary [  41.277s] 8922 tests run: 8921 passed, 1 failed, 15 skipped
error: test run failed
== mut_b rc=100 wall=44s
FAIL [   0.015s] cli::inside_wildcard a_wide_declaration_with_a_fitting_value_keeps_the_pre_compare
     Summary [  40.985s] 8922 tests run: 8921 passed, 1 failed, 15 skipped
error: test run failed

## mut_b re-run (first run reused mut_a build: wall 44s, same single FAIL)
rc=100
FAIL [   0.021s] cli::inside_wildcard constant_wildcard_reads_the_left_operand_at_the_common_width
FAIL [   0.022s] cli::inside_wildcard constant_wildcard_eq_operator_and_sets
     Summary [  42.167s] 8922 tests run: 8920 passed (1 leaky), 2 failed, 15 skipped
K1: | pre L=0 | post3b L=0 | mutb L=1 | iverilog L=0
K2: | pre L=1 | post3b L=0 | mutb L=1 | iverilog L=0
K3: | pre L=0 | post3b L=1 | mutb L=1 | iverilog L=1
