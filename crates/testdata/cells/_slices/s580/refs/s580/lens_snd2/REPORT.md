# lens:soundness round 2 — slice §2 S inside wildcard — POST2 = cf5076ef (md5 09ab4bd248b02f4675f532e2d342282e)
status: COMPLETED (verdict FINDINGS) · calls 29 · designs 10 · mutants 2
## SS1 unsized_xz_lits key — pending
## SS2 domain parity — pending
## SS3 OR-form deviation — pending
## SS4 holds_xz_wildcard — pending
## SS5 lower_size_ctx — pending
## SS6 inside_value_cmp signature — pending
## SS7 mutation — pending

## [update after P1-P4 + mutants] (calls 17, designs 4, mutants 2)
- FINDING A (BLOCKING, new in round 2): OR form + FILL pattern on an absolute hier LHS. P1: POST2 `t.uL.v36 ==? 'x` 0, `==? 'z` 0, `!=? 'x` 1,
  `t.uL.w36 ==? '1` 0, `t.uL.v8 ==? '1` 0, `t.uL.s8 ==? '1` 0 (all 3 lanes); iverilog direct `1 1 0` / `1 1 1`; PRE and POST E3009 (whole file).
  `t.uL.v36 inside {'x}` / `{'z}` POST2 0 0, PRE 0 0, POST E3009. Local known-width twins L1 = iverilog. Chain: wildcard_eq.rs:23-25 fill sized
  `ir_bits_of(lhs).unwrap_or(32)` -> :262-263 OR form -> :379 guard reads unsized_xz_lits only (IntLit arm returns before insert for a fill,
  expr_main.rs:44-48) -> W/P|W at 32 bits, region zero-extends.
  Control C1 plain `t.uL.w36 == '1`, `t.uL.v8 == '1`: PRE/POST2 0 0, iverilog 1 1 -> pre-existing `==` class (not this slice).
- FINDING B (domain split; runtime wrong): P2 both-signed, aw >= pw, sign-sensitive LHS internals: runtime RI `0 0 0 0 0` / RQ `0 0 0 0` on 3 lanes;
  constant KI `1 1 1 1 1` / KQ `1 1 1 1`; iverilog direct, sv2v->iv, verilator all 1. Control RZ/KZ `1 0` = oracles (engine propagates region sign).
  Chain: wildcard_eq.rs:269 both_signed only when aw != pw; :334 sext_lhs = both_signed && aw < w -> mask/clean unsigned -> unsigned region.
  PRE/POST attribution for the runtime cells: pending (-DNOK).
- SS3 (P3): OR form, absolute paths, 19 `==?` + 4 inside cells (signed/unsigned, narrower/wider, x LHS, 68-bit, expr with hier operand, >>>):
  POST2 native = interp = vm = iverilog direct = sv2v->iv. CLEAN for non-fill patterns.
- SS1 (P4): unsized `'bx1` vs sized `32'bx1` against 36-bit v in always / cont assign / size cast / function / pkg function / class method /
  task / generate / paren / ternary / !=?: POST2 = POST = IEEE (1 vs 0). sv2v->iv prints 0 for 'bx1 (sv2v drops §5.7.1 padding; not an oracle here).
- SS7: mutant a (const-domain fill -> Some(false), both sites) rc=100, 1 FAILED constant_wildcard_eq_operator_and_sets: KILLED.
  mutant b (const_wildcard_i64 LHS at own width) rc=100, 2 FAILED (+constant_wildcard_reads_the_left_operand_at_the_common_width): KILLED.

## [update after P2-NOK, P5-P8] (calls 25, designs 8)
- B attribution (P2 -DNOK, runtime only): `(s4+s8) ==? 4'sb1?00` PRE 1 / POST 0 / POST2 0 / oracle 1; `(s4+s68) ==? 4'sb1?00` 1/0/0/1 -> BLOCKING
  (PRE right, regressed at e5147442, unfixed). `(s4+s8) ==? 8'sb1111_1?00`, `(s8b>>>1) ==? 8'sb1111_1?00` 0/0/0/1 pre-existing.
  inside RI 5 cells PRE x / POST 0 / POST2 0 / sv2v->iv 1, verilator 1.
- SS2 edges (P5): i64 at w=64 (signed/unsigned, unsized x pad, fill, CS4 vs 64-bit signed/unsigned pattern), wide at w=68, x/z left
  operands (x under dc -> 1, definite mismatch past an x -> 0): KE = RE, KW = RW = iverilog direct on 3 lanes; inside twins = IEEE. CLEAN.
- FINDING C (NON-BLOCKING, new in round 2, correct->loud): P6 generate-case label `(4'bx100 ==? 4'b1?00)` / `(4'bx100 inside {4'b1?00})`, S=1:
  PRE "G1 default", POST "G1 default", POST2 E3010; iverilog direct (==?) and sv2v->iv (both) "G1 default". Folding label (FOLD) = item in all.
- A breadth (P7): also cont-assign `t.uL.v36 ==? 'x` 0, `t.uL.f36() ==? 'x` 0, `t.uL.g[0].gv ==? 'x` 0, relative `uL.g[0].gv ==? 'x` 0,
  `uL.g[0].gv inside {'z}` 0; iverilog direct 1 each. Controls: relative recorded-shape `uL.v36 ==? 'x` 1, sized `t.uL.v36 ==? 36'hx` 1, `'hx` E3009.
- FINDING D (BLOCKING, new in round 2): P8 -DREALQ `t.uL.r ==? 4'b1?00` (real r=12.0, absolute path): POST2 `RQ 0` rc=0 on 3 lanes;
  PRE E3009, POST E3009; iverilog direct "==? operator may only have INTEGRAL operands"; sv2v->iv "| operator may not have REAL operands".
  REALI `t.uL.r inside {4'b1?00}`/`{4'b11?0}`: PRE 0 0, POST E3009, POST2 0 0; local real twin 0 0 PRE/POST/POST2; no oracle -> UNVERIFIED.
- SS1 mixed lists (P8 MIX): `{'bx1,'1}` 1, `{'1,'bx1}` 1, `{32'bx1,'bx1}` 1, `{32'bx1,'0}` 0 = IEEE (= POST). CLEAN.

## [update after P9, P10] (calls 29, designs 10)
- D extends to LOCAL operands (P9): `string s="a"; s ==? 8'b0110_000x` / `8'b0110_001x` POST2 `1 0` (3 lanes), PRE/POST E3009
  "left operand of unsizable width", iverilog "==? operator may only have INTEGRAL operands"; `$sformatf("%s",s) ==? 8'b0110_000x` POST2 1,
  PRE/POST E3009, iverilog refuses. Whole unpacked array / queue: E3009 in all (refused earlier). `s inside {...}`: StrCmp route, PRE-identical 0.
  Premise refuted: wildcard_eq.rs:249-250 "no width = absolute hierarchical reference"; ir_bits_of None also = String net, string sysfunc,
  unrecorded relative generate path (P7 `uL.g[0].gv`).
- A inside-route chain: expr_ctx.rs:860-863 `w = sibling_ctx(base, l)` -> expr_ctx.rs:990-995 `base.max(self.ir_bits_of(sibling).unwrap_or(32))`.
- SS2 parameter sign (P10): int / unsigned-typed / untyped signed / signed-typed params and an unsigned localparam with a negative init,
  `==?` and inside: constant = runtime (3 lanes) = iverilog direct = sv2v->iv = verilator. CLEAN.
- SS5: map_binop (expr_ctx.rs:74) is a pure match; its only effect is the debug_assert on WildEq/WildNe; every use converted to `irop()`
  (closure type forces it) at node-building sites -> release unchanged. Code-read, no probe.
- SS6: diff = `el: &ast::Expr` removed, `is_unsized_literal(pat)` -> set lookup; xz_literal_reaches_value branch not in the diff;
  callers expr_main.rs:555 `inside_value_cmp(lhs, rhs)` and expr_ctx.rs:870 `(l, r)` identical. CLEAN except A's 32-bit fill default.
## Verdict: FINDINGS — A BLOCKING (new r2), D BLOCKING (new r2), B BLOCKING (r1 regression unfixed, same class as r1 S3), C NON-BLOCKING (new r2).
