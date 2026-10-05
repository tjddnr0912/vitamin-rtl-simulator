# lens:differential round 1 — s580 (inside ==? wildcard)
status: COMPLETED (tool calls 34, designs 16 incl. derived variants P05g P06v P08b P10b P10c)
verdict: FINDINGS — 1 BLOCKING (F1), 4 NON-BLOCKING (F2-F5), notes
binaries: PRE 9d37b3cdbd7f325fe273241dfbb3f465, POST 65a8ab183a63664b3b26ac4607967a2c (md5 verified)
harness: lens_diff/h.sh (vita PRE/POST inside variant, PRE/POST -DIV ==? variant, iverilog -DIV), join.py

## F1 BLOCKING — const-domain inside reads the LHS self-determined against a wider pattern (PRE loud -> POST silent-wrong)
probe: lens_diff/P03.sv (default variant)
| cell | PRE | POST | iv `==?` twin | sv2v->iv (inside) | verilator (inside) |
| L09 `localparam L09 = (4'd15 + 4'd1) inside {8'b0001_?000}` | E3009 | 0 | 1 | 1 | 1 |
| L10 `(4'hF << 1) inside {8'b0001_111?}` | E3009 | 0 | 1 | 1 | 1 |
| L12 `(~4'b0011) inside {8'b1111_11?0}` | E3009 | 0 | 1 | 1 | 1 |
| L14 `(U4 + 4'd4) inside {8'b0001_0?00}` U4=4'b1100 | E3009 | 0 | 1 | 1 | 1 |
| G09 generate-if `(4'd15 + 4'd1) inside {8'b0001_?000}` | E3010 | else | then | then | then |
runtime twins on POST (R09 R09v R10v R12v R14v $display; W09 W14 W12 continuous assign) = 1 = all oracles -> const/runtime split.
`==?` operator twins: PRE 0 / POST 0 (pre-existing const-domain ==? silent-wrong, same evaluator).
mechanism: const_str.rs:197 `inside_wild` routes InsideEq into the masked compare; const_str.rs:206 `let a = self.const_int_selfdet(lhs)?;`
reads the LHS at its SELF width; §11.6.1 Table 11-21 sizes ==/==? operands to max(L(i),L(j)) (sv2v spells it `((4'd15 + 4'd1) | 8'b00001000) == 8'b00011000`).
class: real gap; new in slice (inside), pre-existing (==?).
F1 correct->silent-wrong instances (lens_diff/P07.sv): PRE right, POST wrong, iverilog ==? twin + sv2v->iverilog (inside) agree with PRE:
| L20 `localparam L20 = (4'd15 + 4'd1) inside {8'b0000_?000}` | PRE 0 | POST 1 | iv 0 | sv2v->iv 0 |
| L21 `(4'hF << 1) inside {8'b0000_111?}` | 0 | 1 | 0 | 0 |
| L22 `(~4'b0011) inside {8'b0000_11?0}` | 0 | 1 | 0 | 0 |
| G20 generate-if on the L20 expression | else | then | else | else |
| AB `logic [(4'd15+4'd1) inside {8'b0000_?000} ? 7 : 3 : 0] ab; $bits(ab)` | 4 | 8 | 4 | 4 |
| R20 same expression at run time | 0 | 0 | 0 | - |
| OV `sub2 #(.W(((4'd15+4'd1) inside {8'b0000_?000}) ? 8 : 2))` (P10c) | W=2 | W=8 | 2 | 2 |
more loud->wrong (P10b): L30 `(-(4'd4)) inside {8'b1111_1?00}` PRE E3009 POST 0 iv 1 sv2v 1; L31 `(4'd4 - 4'd5) inside {8'b1111_111?}` PRE E3009 POST 0 iv 1 sv2v 1; runtime twins R30 R31 POST 1.
second slice change in the chain: const_wide.rs:888 `if matches!(op, InsideEq) && bp_any_unknown(..) { return None; }` removes fold_region's width-aware definite-mismatch answer that PRE returned (L20 PRE 0), handing the cell to the self-width masked compare.

## F2 NON-BLOCKING pre-existing — parameter override / -G drop an x/z bit to 0 silently (inside element then compares wrong)
P07.sv: `sub #(.P(4'b1x00)) u1(.v(4'b1100))`, sub: `assign o = v inside {P}; $display(P)`: vita PRE/POST PV1=1000, K1a 0, K1c (`v inside {u1.P}`) 0; iverilog PV1=1x00, `v ==? P` 1, K1c 1; sv2v->iv K1a x, K1c 1. hand-IEEE 1.
P05g.sv: `vita -G "TP=4'b1x00"` prints TP=1000 (PRE and POST), Z14 0; iverilog -P rejects the x digit (loud). class: real gap, pre-existing.
## F3 NON-BLOCKING pre-existing in fix path — `let LU = 'bx1;` element vs 36-bit LHS
P07.sv -DLETT: v36=36'hF_0000_0001: `v36 inside {LU}` PRE 0 POST 0; same literal written directly `v36 inside {'bx1}` POST 1 (= iverilog `v36 ==? 'bx1` 1, census E05q). iverilog/sv2v cannot run let; hand-IEEE §11.12 substitution -> 1. vita self-inconsistent. mechanism: wildcard_eq.rs:252 `is_unsized_literal(pat)` reads the element AST (a let reference), so the 32-bit const pads with compared zeros.
## F4 NON-BLOCKING pre-existing — constant unpacked-array element with x/z stays `==`
P07.sv -DARR: `localparam logic [3:0] CAX [0:1] = '{4'b1x00, 4'b0000}; v1100 inside {CAX[0]}` PRE x POST x (K8v prints 1x00); sv2v->iverilog 1; hand-IEEE 1. Unrecorded instance of residue 1/2 class (element not one Const at lowering).
## F5 NON-BLOCKING — inside with an x/z element on an ABSOLUTE hierarchical LHS is now loud (correct->loud on definite-mismatch cells)
P09.sv: module t { logic [7:0] u8n; late uL(); } (uL.u8 = 8'b0101_0100, u8n = 8'b0000_0100)
| cell | PRE | POST | iverilog ==? twin |
| A1 `wire a1 = t.uL.u8 inside {4'b?100}` | 0 | E3009 unsizable width | 0 |
| P1 `$display(t.uL.u8 inside {4'b?100})` | 0 | E3009 | 0 |
| A2 `wire a2 = t.u8n inside {4'b?100}` | x | E3009 | 1 |
| P2 `$display(t.u8n inside {4'b?100})` | x | E3009 | 1 |
relative `uL.u8` (P3, P08 H01-H03) sizes fine; P4 `t.uL.u8 inside {8'b0101_0100}` (no x/z) unchanged. `==?` on absolute paths already loud on PRE.
P08.sv H04/H05 (`t.u8n` / `t.s8n` vs 4'sb?100): same E3009 on POST; PRE 0 (right) / x.
mechanism: wildcard_eq.rs:206 `let Some(aw) = self.ir_bits_of(lhs_id) else { error … }` — the absolute-path placeholder has no width when inside_value_cmp runs.
## D4 contexts (P06/P06v, 26 cells: do-while, foreach, repeat, nested inside, $countones, $sformatf, delayed assign, port expr, localparam array bound, generate case, case selector, index, assignment pattern, unique if, disable, while/break, always_latch, for-init, +=, interface fn/task, force, fork/join_any, final): POST = verilator on 25 (verilator refuses the x/? generate-case; hand-IEEE `one` = POST). PRE loud (LB localparam, generate case). CLEAN.
backends interp/vm/native byte-identical on POST for P01 P02 P04 P06.
## D6 staged: POST vcmp->velab->vrun output == one-shot (md5 db9daac6…); PRE .vu -> POST velab rc=2 E9002 (loud); POST .vu -> PRE velab rc=2. NOTE: E9002 text says "sim-ir type shape changed … rerun `velab`" for a stale .vu (census item 11, pre-existing wording).
## D5 parity (P08b): const `==?` L01 `S ==? 4'sb?100` PRE/POST 0, iv 1; L02 PRE/POST 1, iv 0 (residue 6); L20 `(4'd15+4'd1) ==? 8'b0000_?000` PRE/POST 1, iv 0; L09 PRE/POST 0, iv 1 (F1 mechanism, pre-existing in ==?, NOT in REPORT residues). Runtime `==?` on the same S/S2 (P03 R01/R02) POST = iv -> new const/runtime split on ==? (const side pre-existing).
P11 (size cast 8'(…), replicate, $signed, index, ternary ctx, nested ==?, side-effect LHS): POST = verilator on 12/12. CLEAN.
ctx-arm unsized/x-MSB (P08b W01-W08, Q01-Q02 NBA, H01-H03 hier): POST = iverilog. CLEAN.
## D1 element kinds (P05): no-x/z kinds (package param, enum, const struct member, const array elem, const fn, $bits, $clog2, paren, signed param, genvar, top param) PRE==POST. x/z kinds: defparam, package param, struct const, div-by-zero localparam loud PRE+POST; class static fn parse error (unsupported); let sized OK (POST 1); override/-G -> F2; let unsized -> F3; array -> F4.
## D3 widths (P04, 41 cells 1..129 bits, unsized 'bx1 'bz1 'h? 'dx 'd? 'b? 'sbx1, fills, upper-word-only x): POST == iverilog ==? on every cell. CLEAN.
## D2 ctx/NBA/class/queue/hier path (P02, 28 cells): POST == iverilog. CLEAN.
## D2 $display path (P01, 57 cells: N/K/W sign-extension x 23 LHS kinds, !=?, x sign bit) — POST == iverilog ==? on every cell (inside and ==?/!=?). CLEAN.
## D5 const vs runtime (P03): F1 above; signed x/z pattern in const domain loud on POST (L01 L02 L13 inside; L07 L08 PRE ==? loud too); real LHS loud.

## not covered / next
- class parameter element (vita parse refuses static class members); interface parameter element; string LHS with x/z element (routes to StrCmp before inside_value_cmp per expr_main.rs:509-535, not probed).
- F2 binder location not found (UNVERIFIED mechanism). F3 mechanism inferred from code + K5b/K5c contrast, not instrumented.
