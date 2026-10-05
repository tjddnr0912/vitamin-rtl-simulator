# lens:soundness round 1 — slice §2 inside ==? (e5147442)
status: COMPLETED (verdict FINDINGS)
tool calls: 41 · designs: 12 · mutants: 2

## S1 site census
(pending)
## S2 partition
(pending)
## S3 sign source
(pending)
## S4 lowering once
(pending)
## S5 const domains
(pending)
## S6 map_binop WildEq release
(pending)
## S7 constraints
(pending)
## S8 serialization
(pending)
## Findings
(none yet)

## [update after S5-probe P1] FINDING F1 (BLOCKING, new-in-slice): const-domain `inside` sizes the LHS at its OWN width
- chain: const_fn_width.rs:552 `const_compare_special(*op, lhs, rhs)` (before the width-honest pair sizing below it)
  -> const_str.rs:197 `inside_wild` admits InsideEq + sized unsigned x/z literal
  -> const_str.rs `let a = self.const_int_selfdet(lhs)?;` (LHS at its self width, not max(L,R) of §11.6.1 Table 11-21)
  -> masked compare `(a & !mask) == (pat & !mask)`.
- probe probes/P1.sv: PRE E3009/E3010 (loud) ; POST exit 0:
  L1 `(4'd15+4'd1) inside {5'b1?000}` POST 0, sv2v->iv 1 ; L3 `(~4'b0000) inside {5'b1111?}` POST 0, iv 1 ;
  L5 `(A+4'd1) inside {5'b1?000}` POST 0, iv 1 ; L7 `(4'b1000<<1) inside {5'b1?000}` POST 0, iv 1 ;
  generate-if same cond: POST "G1 else", iv "G1 then" (PRE E3010).
  range bound `[(...inside...):0]` POST $bits 1, iv 2 (PRE value pending).
  runtime twins R1..R9 POST 1 = oracle (runtime path is right).
- `==?` operator twin L2/L4/L6: POST 0, iverilog direct 1, sv2v->iv 1 (PRE value pending: same code path => pre-existing).

## [update after P2a/P2b/P2c/P3]
- F1 attribution: `==?` operator twin (P2a) PRE = POST = L2 0, L4 0, $bits 1, "GQ else"; iverilog direct / sv2v->iv / verilator: 1 1 2 "GQ then" => `==?` const domain PRE-EXISTING silent-wrong (same mechanism, const_str.rs:206).
- F1 more consumers (P2c): `localparam bit LB = (4'd0-4'd1) inside {5'b1111?}` PRE E3009 / POST 0 / sv2v->iv 1 / verilator 1 ;
  instance override `#(.P((4'd15+4'd1) inside {5'b1?000}))` PRE E3009 / POST P=0 / iv 1 / verilator 1.
- range bound with inside (P2b): PRE $bits 1, POST 1, sv2v->iv 2, verilator 2 => pre-existing value, unchanged.
- S6 (P2a, P3): release PRE/POST `8'(a + (b ==? 4'b1x0x))` = iverilog direct (b=1000: 00000010 both). `map_binop` result is unused on the
  comparison arm of lower_size_ctx (`_ => lower_size_leaf`), so no release silent-wrong. `8'(a + (b inside {4'b1x0x}))` PRE xxxxxxxx -> POST 00000010 = verilator.
- S3 (P3): `==?` and inside with signed pattern wider than a signed LHS for Call / int vs 64- and 65-bit / ternary / unary minus / array element /
  class method / unsigned pattern: POST = iverilog direct on interp, vm and native lanes. CLEAN for those kinds.
- F2 (NON-BLOCKING, pre-existing): parameter OVERRIDE value with x/z is silently zeroed: `sub2 #(.P(4'b1x00))` prints P=1000 (PRE and POST);
  iverilog direct P=1x00 eq=x wq=1; verilator P=1x00 wq=1 inside=1; vita `v inside {P}` 0, `v ==? P` 0, `v == P` 0. Refutes the S2 partition claim
  for a constant x/z element reaching `inside` through an override (no wildcard, no loud).

## [update after P4a/P5/P7/P8 + debug binary]
- F1 STRONGER (PRE right -> POST silent-wrong), P7: `localparam L4 = (4'd15+4'd1) inside {5'b0?000}` PRE 0, POST 1, verilator 0;
  generate-if same cond PRE "G4 else", POST "G4 then", verilator "G4 else". `==?` twin L4q: PRE 1, POST 1, iverilog direct 0, verilator 0 (pre-existing).
- F3 (NON-BLOCKING, pre-existing): range bound swallows a DECLINED x/z element into a 1-bit net (P4a): `[(4'b1100 inside {4'sb1?00}):0]` PRE 1 POST 1,
  `{'b1?00}` 1/1, two x/z elements 1/1; sv2v->iv and verilator 2 for all. `==?` twins `'b1?00` and the `||` pair: PRE 1 POST 1, iverilog direct 2.
  Refutes REPORT "range bound ... right or loud" for declined shapes. Admitted shape `{4'b1?00}` PRE 1 -> POST 2 (fixed).
- F4 (NON-BLOCKING, new-in-slice, correct->loud) P8: `4'b0100 inside {4'sb1?00}` PRE 0 -> POST E3009; `4'b0100 inside {'b1?00}` PRE 0 -> E3009;
  100-bit `W inside {4'b000?}` PRE 0 -> E3009; generate-if `4'b0100 inside {4'sb1?00}` PRE else -> E3010. sv2v->iv and verilator: 0 0 0 else.
  Mechanism: const_wide.rs:888 decline precedes wide_eq_with_unknowns, whose definite known-bit mismatch (0) is also the `==?` answer.
- S4 fill: `w36 inside {'x}` / `{'z}` / `{36'h0,'x}` (36-bit LHS) PRE x x x, POST 1 1 1 = verilator, iverilog `w36 ==? 'x` 1. CLEAN.
- S6 debug: POST debug binary (md5 e429b343..., = recorded) on P2a `8'(a + (b ==? 4'b1x0x))`: rc=101 panic expr_ctx.rs:101:13
  "WildEq must be lowered via lower_wildcard_eq" (pre-existing, debug-only). P2b inside under size cast: debug rc=0, no panic.
- S8: PRE .vu -> POST velab rc=2 E9002; POST .vu -> PRE velab rc=2; POST vcmp->velab->vrun A=1 B=0 C=1 (PRE x 0 1). No other file in crates/ tests/
  embeds the old or new hdl-ast hash (decimal, hex, raw bytes). CLEAN (E9002 text says "sim-ir type shape" for an hdl-ast mismatch: cosmetic, known residue 11).

## [update after P9 + S1/S7 census]
- S3 ctx path (P9, continuous assign): c1..c5 / q1..q4 POST = iverilog direct (q) and sv2v->iv (c) on native, interp, vm. PRE x/0. CLEAN.
- S7: crv.rs:43 `B::Eq | B::InsideEq => C::Eq`, crv.rs:150 bound narrowing, crv.rs:266 list, crv.rs:137 flip `other => other`, netdecl.rs:851,
  expr_special.rs:42/56 — InsideEq beside Eq at every constraint site. CLEAN by census.
- F3 chain: const_eval.rs:1074 `nonconst_bound_reason` names no reason for literal operands ("literals · Call · New · Dollar · Error"), so the
  declined bound keeps a 1-bit net silently (pre-existing; PRE identical).
- S1: every `BinOp::Eq` / `B::Eq` / glob `Eq` site in elaborate + hdl-parser lists InsideEq or is a constructor/token map; catch-all sites do not
  name Eq either. cli/src and sim-engine/sim-ir do not match on hdl_ast::BinOp. Only value-WRONG site: const_str.rs:206 (F1).

## [mutants] copy = lens_snd/src (git archive e5147442), CARGO_TARGET_DIR=lens_snd/target, `cargo nextest run -p cli --test inside_wildcard --locked --no-fail-fast`
- A (wildcard_eq.rs:221 `Some(s) => s.signed` -> `Some(_s) => false`): rc=100, 18 passed / 2 FAILED
  (signed_comparison_extends_by_sign: `ss-narrow-pat-msbq 0`, `ss-narrow-lhs-sext 0`; wildcard_eq_operator_extends_by_sign_and_unsized_x). KILLED.
- B (`if true { return None; }` at the top of inside_value_cmp): rc=100, 6 passed / 14 FAILED. KILLED.
- Survivors on both: constant_contexts_fold_the_wildcard (const domain does not use inside_value_cmp); POST passes the whole file, so no test pins F1.

## Verdict: FINDINGS — F1 BLOCKING (new); F2, F3 NON-BLOCKING pre-existing; F4 NON-BLOCKING new (correct->loud); N1 NOTE (debug-only panic, pre-existing).
