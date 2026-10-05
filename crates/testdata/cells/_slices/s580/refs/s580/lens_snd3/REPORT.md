# lens:soundness round 3 — slice §2 S (inside x/z -> ==?), delta cf5076ef..e6c9cc8d
status: COMPLETED · verdict FINDINGS · calls 14 · designs 5 (p1,p3,p4,p5,p6 in p/) · mutants 1 · term completed
POST3 76ad4738a0ec5a26cc7718c675467a9e · POST2 09ab4bd248b02f4675f532e2d342282e · PRE 9d37b3cdbd7f325fe273241dfbb3f465
Harness: h3.sh (POST3 native/interp/vm, POST2, PRE, iverilog 13 direct, sv2v->iv, verilator) + join3.py

## R3-A BLOCKING (new in round 3; created by the R3-3 narrowing of round-2 C)
generate-case label: i64 fold declines, 4-state fold answers a KNOWN value -> label skipped -> default, rc=0.
generate.rs (e6c9cc8d) R3-3 `if lv.is_none() && holds_xz_wildcard(lab) && fold_self_bits(..).is_none()` refuses only on fold
ABSENCE; a Some(known) falls to `if lv == Some(scrut)` with lv None -> never chosen.
| cell | scrut | label | POST3 x3 | POST2 | PRE | iv direct | sv2v->iv |
| H8 | 1 | $isunknown(4'bx100 ==? 4'b1?00) | default | E3010 | default | item | item |
| J4 | 0 | (4'bx100 ==? 4'b1?00) >> 1 | default | E3010 | default | item | item |
| J7 | 1 | $isunknown(4'bx100 inside {4'b1?00}) | default | E3010 | default | sorry(inside) | item |
| J8 | 1 | 2'd2, $isunknown(4'bx100 ==? 4'b1?00) | default | E3010 | default | item | item |
Fold value itself right: JV `localparam PV = $isunknown(4'bx100 ==? 4'b1?00)` POST3 1 = iv 1 = sv 1; runtime 1.
Same-shape labels the i64/4-state path DOES answer: H1 x||1, H2 x&&0, H3 x===1'bx, H7 x!==1'bx, J5 |{x,1}, J6 !$isunknown(x) -> item = oracles.
Loud (safe): H4 x?1:1, J1 $countbits(x,1'bx), J2 x|1, J3 x&0, G4 "ab"==?, G9 string-literal param -> E3010 (PRE default).

## T1 CLEAN — 89 runtime cells (p1): POST3 native=interp=vm = sv2v->iv = verilator on all; iv direct (-DNOC -DNOI, 71 cells) = POST3.
Engine sign source = same rule: sim-engine/src/width.rs:155-177 build_with uses sim_ir::selfwidth with call_ret (ret_signed) + class_fields,
as packed.rs canonical_self_width (func_metas, class_field_widths, hier_placeholder_shape).
## T2 (a) wide/string candidates: G1,G2,G3,G5,G6,G7,G8 item = oracles; G4,G9 loud. Hole found via x-absorbing operators (R3-A).
## T2 (b) fold_self_bits ==?/InsideEq arm: w = max(l0.1, r0.1), sg = l0.2 && r0.2, narrower side refolded via fold_region(..)? (declines on None).
Carry: (PF+64'h1) ==? 65'h1_0000_0000_0000_000? -> L6 1, R6 1, G6 item = iv = sv. CLEAN.
## T3 CLEAN — no or_form left; wildcard_cmp_ids callers wildcard_eq.rs:54, :93 (<- expr_main.rs:555, expr_ctx.rs:870).
E3009 reached: K1 abs path, K2 string, K3 $sformatf, K5 abs real, K6 abs inside, K7 abs real inside, K8 uL.g[0].gv, K9 abs + 'x.
K4 local g[0].gv: 1 = iv. POST2 values K1/K8/K9 1 (=iv), K2/K3 1 (iv refuses), K5 0 (iv refuses); PRE E3009 except K6 x, K7 0.
## T4 mutant `if aw != pw && cv.signed` — KILLED: rc=100, 28 run / 27 pass / 1 FAIL signed_comparison_signs_the_left_operand_operators
(inside_wildcard.rs:1213, left ["R 0 0 0 0 0 1 0 1 1", ...]); built from lens_snd3/src (Compiling sim-ir/elaborate/cli, 21.62s).
## Notes
- N1 K7 abs real inside: PRE 0 / POST2 0 / POST3 E3009, no oracle (iv sorry on inside) -> UNVERIFIED value->loud.
- N2 iverilog direct vvp SIGABRT (rc=134) on full p1 (class/queue/inside cells); rerun without them clean.
- N3 const function `a ==? 4'b1?00` in a localparam: E3009 in PRE/POST2/POST3 (loud).
