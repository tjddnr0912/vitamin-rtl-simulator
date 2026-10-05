# lens:soundness round 1 — REPORT (live, updated per question)

status: started
binaries: (to fill)

## Task 1 all-sites census
(pending)
## Task 2 gen_case_choose decision table
(pending)
## Task 3 population paths
(pending)
## Task 4 holds_xz_wildcard guard traversal
(pending)
## Task 5 fold_in_region refactor identity
(pending)
## Task 6 mutants
(pending)
## Findings
(none yet)

## FINDING F1 (BLOCKING, half T, new instance of §4.5.568 root "wide_name_bits is not a scope model")
genvar under a same-named NON-FITTING wide constant: bit domain reads the stale wide_param_bits entry.
probes: lens_snd/probes/A1S.sv A1L.sv A2S.sv A3S.sv (A1D = PRE-EXISTING run-time twin)
A1S: PRE zero/one = iv = sv2v = vl ; POST-T and POST def/def (exit 0)
A1L: PRE def/a = iv = sv2v = vl ; POST-T and POST def/def
A2S (wildcard import pk::W + genvar W): PRE zero/one = iv = vl ; POST-T/POST def/def
A3S (gen-block [127:0] k + genvar k): PRE zero/one = 3 oracles ; POST-T/POST def/def
census cells NA06L/NA06S/G01 used 65'd9, which FITS i64 -> stored narrow in params, never in wide_param_bits.
F1 more producers (same root: a narrow rebinding at a key whose stale wide_param_bits entry wide_name_bits asks FIRST):
B2E (local enum label E1 under wildcard-imported pk::E1 65-bit): PRE a = iv = sv2v = vl ; POST-T/POST def
B3X (explicit import pk::W + genvar W): PRE def/a = iv = vl ; POST-T/POST def/def
B4P (`(i ==? 32'b???1)` label, genvar i under wide i): PRE def/odd = POST-T = iv = sv2v ; POST def/def  -> S half exposes it (wide ==? arm)
B1U/B1N (loop-iteration wide->narrow same-named localparam): all equal oracles (keys distinct per iteration) - clean

## FINDING F2 (NON-BLOCKING, half T, NEW mechanism; refutes claim 4 phase consistency)
eager region over ALL labels: a LATER label that forward-references a generate-block localparam is unfoldable in the
Nets phase (region None -> label 1 keeps pre()) and foldable in later phases (region Some -> label 1 decided by bits).
D1/D2 (`case (-1) 32'hFFFFFFFF: … K: … default` / K after default, `localparam logic [7:0] K = 8'd99` after the case):
PRE def 99 (consistent, wrong); POST-T/POST E3010 undeclared net `top.gb[0].g_a[0].w` (Nets=default, later=g_a); iv/sv2v/vl a 1.
D3 (no K label): POST-T/POST a 1 = oracles. D4 (same-named arm blocks) / D5 (arm-only assigns): POST a 1 = oracles (no silent mix found).
## S-half probes C1-C12, E1-E12 (==? over 24 lhs shapes, localparam + bound): no value->loud, no wrong value; loud->value C11 E4 E5 E6 = oracles.

## Task 4 guard traversal (x-valued `(4'bx100 ==? 4'b1?00)` / compound `{2'b1?,2'b00}` pattern), PRE | POST-T | POST | iv | sv2v
G5 packed range        : 1 | 1 | LOUD(guard) | 1 | x        (documented class, oracles split)
G6 unpacked dim (cmpd) : 4 | 4 | LOUD(guard) | 2 | 2        (PRE wrong -> loud)
G9 typedef range       : 1 | 1 | LOUD(guard) | 1 | x
G11 port range         : 1 | 1 | LOUD(guard) | 1 | x
G12 function ret range : 1 | 1 | LOUD(guard) | 1 | x
G13 localparam range   : 32| 32| LOUD(guard) | 1 | x
G1/G8 replication count: LOUD all vita | iv err / sv2v err (G8 cmpd: oracles 0001)
G3 size cast           : LOUD all vita | iv err | sv2v 1
G4 gen-for condition   : LOUD all vita | both err
G7 localparam int value: LOUD all vita | iv 0 | sv2v x
G2b indexed part-select WIDTH: y=0001 exit 0 on PRE/POST-T/POST | iv + sv2v REFUSE -> PRE-EXISTING silent route the guard does not cover (finding F3)

## Task 6 mutants — battery 1 note
mut_a valid (81 Compiling lines, 1m33s build): KILLED by cli::generate_case_label_domain a_split_sizing_keeps_the_i64_answer
  + cli::param_default_takes_declared_type a_generate_case_string_label_compares_as_an_i64 (8928 run, 2 failed).
mut_b/mut_c/(mut_d) battery-1 runs INVALID: "Finished in 0.30s", 0 Compiling lines -> they ran mut_a's binaries
  (shared CARGO_TARGET_DIR + path-relative metadata + git-archive mtimes = commit time older than mut_a's fingerprint).
Re-run with the mutated files touched first: mut_run2.sh -> mut2_*.log

## Attribution (REF = eab81ef2, S without T): A1S A1L B2E B3X B4P D1 all = PRE on REF -> F1/F2 belong to the T lane
(B4P needs both: T's bit-domain label lane + S's wide `==?` arm). PRE-EXISTING twins of F1's root (shared resolver):
A1D: `$display(i)` = 18446744073709551625, `localparam int K = i` = 9 (oracles 0/1) on PRE/REF/POST-T/POST
B2D: `$display(E1)` = 18446744073709551616, `localparam int K = E1` = 0 (oracles 1/1) on PRE/REF/POST-T/POST
H1/H2/H3 (narrow #() override of a wide non-fitting default; instance array): all = 3 oracles on PRE/POST-T/POST (default never installed) - clean

## Task 2 decision table (gen_case.rs:62-131, read from e350ef79 source)
row | s0 (case expr, known bits) | every label folds | whole (fold_in_region) | decision                  | vs PRE
R1  | None (declines or x/z)     | -                 | -                      | pre() every label         | identical
R2  | Some                       | no                | -                      | pre() every label         | identical
R3  | Some                       | yes               | None                   | pre()                     | identical
R4  | Some                       | yes               | Some(b), b != pair     | pre()                     | identical
R5  | Some                       | yes               | Some(b), b == pair     | bits decide (pair)        | differs only here
- R5 is oracle-backed only if both readings read the right LEAVES; they share leaves (param_leaf_bits), so a misread
  leaf makes both agree on the wrong value -> F1 (stale wide_param_bits) lands entirely in R5.
- x-valued label in R5: case_equal is 4-state, x != known -> non-match = PRE's skip. no-i64 label in R1-R4 -> non-match.
- phase: R2 <-> R5 switch when a label's foldability differs by phase (forward ref) -> F2.
- side effects of the eager fold: error_at takes &mut self (driver.rs:368) and gen_case_choose is &self; const_wide.rs has
  no borrow_mut/Cell; the only RefCells (const_call_fn, const_call_pkg, lib.rs:509/994) are not touched in const_wide /
  const_array / const_select / scope.rs (grep 0) -> no diagnostic or side effect from labels after the match/default.
Z-probes (generate-case label shapes): Z2 `signed'(4'hF)`, Z3 `$signed(4'hF)` vs case(-1): PRE def -> POST a = 3 oracles (fix);
Z4 Z6 split -> PRE kept; Z5 `case (8'd200) byte'(200)`: def on all vita, a on all 3 oracles = PRE-EXISTING (bit domain has no
type-cast arm, region None -> PRE's i64 -56 != 200).

## Claim 1 re-measure (vita side re-run; oracle text inherited from T_census.tsv)
252 cells re-run on PRE/POST-T/POST: 0 mismatches vs the TSV's PRE and POST-T columns. PRE != POST-T on 52 cells (= 10 + 42);
every POST-T answer equals at least one oracle's recorded answer. POST-T != POST on 6 (T2 T3 X13 X23 M04 R08), each the
iverilog/sv2v answer (verilator refuses x/? labels). The census is sound on its own cells; F1/F2 are OUTSIDE it.
I1-I4 (wildcard common width over untyped / #()-overridden params, incl. a generate-case label): all = oracles - clean
J1-J8 (select / array-element label leaves): J2 `localparam logic [3:0] A [1:0] = '{4'd1, 4'd2}; case (2) A[0]` = def on
PRE/POST-T/POST, a on sv2v + verilator (iverilog: "sorry: unpacked array parameters are not supported yet") = PRE-EXISTING; rest clean.
K1/K2 (`parameter string MODE` / untyped string MODE as generate-case scrutinee, 4 overrides): E3010 scrutinee not constant on
PRE/POST-T/POST, oracles fast/slow/fastest/def = PRE-EXISTING (scrutinee-still-loud class). K3 (logic [63:0] MODE): all = oracles.

## Task 1 census (POST e350ef79 source; grep -rn '<fn>(' crates/elaborate/src minus the definition)
fn                    | callers | sites                                                     | verdict
gen_case_choose       | 1       | generate.rs:447                                           | body slice only; ok
fold_in_region        | 6       | const_wide.rs:953 955 981 982 (w,sg = l0/r0 max/and), gen_case.rs:96 116 | ok (same args as old closure)
fold_selfdet_operand  | 6       | const_wide.rs:876 932 933 1009 1038, gen_case.rs:69       | ok
const_wildcard_i64    | 1       | const_fn_width.rs:559 (only when env/envw empty)          | ok; const-fn bodies keep PRE route
wildcard_match        | 2       | const_wide.rs:969, wildcard_eq.rs:226                     | ok
holds_xz_wildcard     | 1       | const_eval.rs:1039 (check_const_range_bound only)         | doc says "a generate-case item" asks it: STALE (W1)
check_const_range_bound| 8      | packed.rs:191-192, params.rs:1696-1697, array_geom.rs:312-313,321,572-573 | = guard reach
const_eval_in_scope   | 97      | (i64 entry; changed only for WildEq/WildNe/InsideEq w/ x/z literal pattern via eval_const_env_at) | 0/1 unsigned 1-bit
BinOp lists (Eq/Ne/CaseEq windows lacking WildEq/WildNe/InsideEq, const_*/param*/gen*/wildcard_eq/expr_size*): 4 hits —
const_wide.rs:211/224/995 run AFTER the arm maps WildEq|InsideEq->Eq, WildNe->Ne (const_wide.rs:975-979); wildcard_eq.rs:355 builds IR.
binop_result_is_context_determined (const_fn_width.rs:105-107), expr_size_ctx.rs:517-519, override_type.rs:178-180,
wide_top_is_self_determined, const_fn.rs:131/134 (i64 generic: WildEq/InsideEq -> ==, WildNe -> !=): all list the three ops.

## Task 3 population census (writers: grep -rnE '\.<map>\.(insert|remove|entry|...)' crates/elaborate/src)
map             | writers | phase / key                                                         | label-lane hazard
params          | 23 bind_param_value sites (+expr_size_hier 2, frames_reserve 1) | header/body: module elab; generate localparam/genvar/enum: EVERY gen phase by position at fq(scope) | forward refs differ by phase (F2, NA0xF)
wide_param_bits | 8 (package.rs 388 657 998 1001 1272 1277 1386, wide_param_range.rs:56) | imports at module key; wide decls at fq | NEVER cleared by any of the 23 narrow binders (0 of 23 touch it within +-15 lines) and asked FIRST by wide_name_bits -> F1
param_range     | 8 (bind_param_value clears, bind_param_range sets; pkg save/restore) | with params | ok
param_meta      | 28; NOT cleared by bind_param_value; genvar save/restore | | B5/B6 probes clean
pkg_consts/_wide_bits/_const_range/_meta | 1 each (package.rs 1006-1026) | once per package | ok
symbols         | 11 | nets/ports/pkg vars | inner net -> bit domain declines -> pre() (NA09 class)
str_param_raw   | 6  | | string scrutinee still loud (K1/K2, PRE-EXISTING)
genvar          | generate.rs:264/384/394 (+range 0,32 / meta 32,true, real suspended) | every phase at fq(scope of the for) | stale WIDE same-named entry NOT suspended -> F1
generate Import | generate.rs:880 Nets phase only | | NA10L/E class (not re-filed)
stale per-iteration: B1U/B1N clean (per-iteration keys).
mut_b (battery 2, rebuilt: Compiling elaborate/sim-engine/cli, 1m41s): KILLED, 8928 run / 3 failed:
  cli::generate_case_label_domain an_x_valued_label_beside_a_call_case_expression_is_a_non_match
  cli::generate_case_label_domain a_label_the_bit_domain_cannot_read_keeps_the_pre_answer
  cli::inside_wildcard shapes_without_a_constant_value_are_loud
MC1 (`SP=8'sd84; SP ==? 4'sb?100`): PRE 0, POST-T 0, POST 1, iv 1, sv2v 1. MC2 (`SN=4'sd-4; SN ==? 8'sb1111_?100`): PRE/POST-T loud, POST 1 = iv = sv2v.
MD1 (`logic [(4'b1100 ==? {2'b1?,2'b00}):0] v`): PRE/POST-T 1 (silent), POST loud (guard), iv 2, sv2v 2.
mut_c (battery 2, rebuilt, Compiling elaborate/sim-engine/cli 1m40s): SURVIVED, 8928 run / 8928 passed.
  binary used in that run snapshotted at 01:24:25 (vita_snap1 md5 805086ed4375b2e320f7ea18d00528d0, target/debug/vita mtime 01:21:34 = mut_c build window);
  killing cell MC3/MC4: `localparam A = (8'sb1111_1100 ==? 4'sb1?00);` POST 1 / mut_c 0 / sv2v->iverilog 1 (PRE, POST-T loud).
  a NAME lhs (`SQ ==? 4'sb1?00`, `SP ==? 4'sb?100`) gives the POST answer on mut_c -> only a literal-lhs cell pins the i64 sign rule.
mut_d (battery 2, rebuilt 1m41s): KILLED, 8928 run / 1 failed: cli::inside_wildcard shapes_without_a_constant_value_are_loud.
  mut_d binary (md5 54bc2e527169de29bdf5f5af0f9edea8): MD1 1, G5 1, G6 4 (silent, = PRE) -> guard disabled as intended.
mut_a KILLED (battery 1, valid build). Survivors: mut_c only.

## SUMMARY (status: completed, round 1)
F1 BLOCKING (T; S exposes B4P): stale same-key wide_param_bits read first by wide_name_bits -> generate-case takes default
   at exit 0 where PRE = iverilog = sv2v = verilator took the item (A1S A1L A2S A3S B2E B3X; B4P POST only).
   root = §4.5.568 class; premise census: 23 bind_param_value sites, 0 clear wide_param_bits; wide_name_bits asks wide first.
F2 NON-BLOCKING (T): eager region over a later forward-referenced label splits the arm by phase (D1 D2: PRE def -> POST E3010).
F3 PRE-EXISTING: indexed part-select width with a declined x/z wildcard compare is silent (G2b), guard does not reach it.
W1 NON-BLOCKING wording: holds_xz_wildcard doc names a generate-case caller; e350ef79 has only check_const_range_bound.
PRE-EXISTING (not re-filed as new): A1D B2D (run-time twins of F1 root), J2 (unpacked [1:0] array param element), Z5 (byte'() label),
   K1/K2 (string scrutinee loud).
mutants: a KILLED, b KILLED, c SURVIVED (cell MC4), d KILLED.
target dir left in place for the parent: lens_snd/target (not cleaned: cargo clean/sweep forbidden to lenses).
