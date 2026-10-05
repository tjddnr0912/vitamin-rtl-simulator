# lens:differential round 1 — §2 T + §2 S(a)
status: DONE (term: completed)
binaries: (to fill)
## Questions
binaries (md5 -q, re-checked at start): PRE 4ace7617440041d0d16aee5309fded10 · POST-T 37d2b613f937979ade730b6808175e22 · POST 1e41978f78724cc11b691cf6babfe911
harness: lens_diff/run.py (iv / sv2v->iv / verilator / PRE / POST-T / POST; raw output p/<id>.<tool>.txt)
## Q1 T batch 1 (T01-T20): per-instance #() and -G, genvar shadowing a module param, gen-scope localparam shadow, loop-scope 65-bit, nested genvars, $unit/package/import signed binders, typed int/byte/shortint/longint/enum/typedef/struct binders, enums, x/z scrutinee, >64-bit string labels, $signed/$unsigned/$clog2/$bits labels, concat/replication, interface/program, procedural twin, defparam, override chain, ==?/inside labels, selects of packed/unpacked/non-zero-LSB/ascending params
- vita moves (PRE->POST): T05 (3 cells skip->match), T07 Uu (hit->def), T11 SN (def->hit), T12 A B G H L (def->hit): every move = iverilog+sv2v answer.
  - T07 Uu `case (64'hFFFF_FFFF_FFFF_FFFF) U` (U = $unit int -1): iv/sv def, vl hit. vl self-contradicts: T07p `64'hFFFF_FFFF_FFFF_FFFF === U` = 0 and procedural case hit in vl; iv procedural def. POST def correct.
- no vita move elsewhere; pre-existing loud: T09 65-bit enum label E3009 (enum value fold), T14 generate in interface E3009 (MVP), T10 x scrutinee E3010 (documented residue).
- oracle artefacts seen: sv2v types an enum/int-array with no base sign as unsigned (T07p NEGbits signed=0, T20 AU0); verilator sign-extends a signed item in an unsigned case (T07p/T08b/T12 K/T15 -1).
- T21-T23: moves C `case (0) 1 << 32` def->hit (= iverilog), T22 sQ2 `case (Q2) 64'hFFFF_FFFF_FFFF_FFFF / -1` (Q2 = untyped -1) ff->m1 (= iv+sv; vl def, vl self-contradicting sign rule). Everything else unchanged and = iverilog.
- PRE-EXISTING (documented, ROADMAP line 474): `parameter signed Q4 = 4'hF` reads 15 unsigned in every consumer (T22v: $display 15, `Q4<0` 0, generate-if, procedural case, generate-case); all 3 oracles -1. Not this branch.
- harness note: iverilog -P rejects `_` in a value (T02 iv 'WP hit' = WP not overridden); vita -G and verilator -G apply (gchk.sv).
## Q2 S ladder (gen_s.py, 46 pairs x {==?/!=?, inside} = 92 cells; widths 1/4/8/9/16/32/33/64/65/128, signed/unsigned, unsized x, fills, x left, arithmetic/shift/~/concat/?: left, names A4/S4/UP/pk::K8/W65; positions localparam, range bound $bits, generate-if, run-time display)
- ==?: loud->value 30, value->value 4 (Sq27 28 30 31: PRE constant 0 vs run-time 1), unchanged 12. POST = iverilog native ==? on 46/46.
- inside: loud->value 33, value->value 3 (Si19 23 38: sv2v rewrites unsized x pattern, vl = POST), unchanged 10. POST = iverilog ==? twin on 46/46.
## Q3 S positions outside S2_matrix (S01b S03b; S02/S04 loud on all three vita)
- S01b loud->value, POST = sv2v on: array-of-instances count, #() override value, package localparam read by another package (p2 imports p1), ?: on ==?, int localparam of !=?, replication count, $bits of replication, generate-for CONDITION `!(i ==? 4'b1???)` (8 iters), generate-if `i ==? 3'b1?1` (5,7), generate-case scrutinee `A4 ==? 4'b1?00`.
- S03b loud->value, POST = sv2v = verilator: nested inside, real `inside {[1.0:3.0]}` / `{2,3}` / `{3}`, signed range, unsigned-vs-negative range, mixed-width elements, range + x element list.
- unchanged loud on PRE/POST-T/POST: `"a" ==? 8'b0110_000?` (string literal operand), `5 inside {[4:$]}`, unpacked-array set `{AR}`, `let` call in a localparam, any constant function whose body holds `==?` with x/z (S02b/S02c; = matrix CF_*).
## Q4 random generate-case label sweep (gen_r.py seeds 581/7/99; labels over literals 1..65 bits, typed/untyped/65-bit params, unary/binary/shift/compare/logical/$signed/$unsigned/concat/replication/?:/size casts; scrutinees built from each label's own iverilog value at own width, width+3, sign-flipped, 64-bit sign-extended, unsized)
- 1711 cases compared (R*: 317, Ra/Rb: 1394). MOVED 487 = iverilog AND verilator; MOVED-split 39 (all one pattern: signed item vs a wider UNSIGNED case expression; vl sign-extends the item, iv + LRM zero-extend; vl self-contradicts via `===`, T07p) -> POST = iverilog = LRM; same 1035; same-split 148 (designed: pair/whole split keeps PRE).
- 2 R cells with scrutinee an unsized decimal >= 2^31 (3865470567, 137438953471): oracles self-inconsistent (iv procedural `==` 0 vs iv gen-case hit vs vl 1; T25 rt line). harness-format; generator then restricted.
- R03: scrutinee unsized decimal > 2^64 (18446744073709551613) E3010 on all three vita: pre-existing, generator artefact.
- T24 (phase): 65-bit label arm chosen identically for nets ($bits 8), processes and an outside `assign g.w` (POST = iv = vl; PRE took default for all phases). $bits(net) labels before/after the case: unchanged, = vl.
- T26b nested generate-case with wide labels + inner localparams: all agree. T27b loop + instance param per arm: PRE g[2] K=22 wrong -> POST 12 (= oracles).
- T28 $unit / wildcard-import shadowed by module localparam: unchanged right; K2u moved hit->def (= iv, vl sign quirk).
## Q5 re-measure of the T census (252 cells copied to lens_diff/rc, run on PRE/POST-T/POST)
- my PRE/POST-T verdicts = T_census.tsv on 252/252; my POST = S_census_gencase.tsv on 252/252. 194 same, 58 moved; every moved POST equals >= 1 oracle column (oracle columns inherited).
- 34 cells where the inherited oracle columns differ (none ERR): POST = PRE on 31; moved 3 = N09 (vl), NA11G/NA11S (sv2v+vl) = the documented rulings.
## Q6 claim 6 re-run: python3 $SCR/cmp4.py post/vita -> lens_diff/cmp4_mine.txt = {'both': 427, 'pre': 7, 'post3': 181, 'neither': 51}, class lines identical to cmp4_post.txt; all 58 non-REF variants are generate-case designs (r26 X1/X4, K02, lens_diff3 G1-G3, lens_snd3 p3/p5/p6, f2/w65).
## Q7 sweep 2 (casts/reductions/$countones/$onehot/$clog2/**/128-bit/param-width casts; seeds 4242/1717): 1388 cases: MOVED 394 = iv AND vl; MOVED-split 15 (same signed-item/unsigned-wider-scrutinee vl quirk); same 925; same-split 37; same-NOORACLE 17 (POST = PRE, iv = vl differ).
- the 17 all hold a PARAMETER-width size cast `W6'(...)`: minimal T30: `case (6'sh30) W6'(P8)` PRE/POST-T/POST def, iv/sv/vl hit; literal twin `6'(P8)` moves def->hit (= oracles); `{P8, 3'(W6'(3'h3))}` def vs oracles hit. => PRE-EXISTING in the label lane: the bit domain declines a param-width cast, the label keeps the width-blind i64 answer (same root as L07/S44/S48 residues: "a label no bit fold reads keeps PRE").
## Q8 S consumer-position ladder (gen_sp.py: 14 operand shapes x {replication count, size-cast width, enum initializer, part-select width, unpacked dimension, int localparam of !=?, [c:0] bound, #() override, generate-for bound, $bits} + generate-case label/scrutinee = 28 cells): loud->value 25 (POST = iverilog on every line), unchanged 3. No divergence.
## Q9 FINDING candidate: holds_xz_wildcard refuses a range bound whose value is DEFINED (x does not reach it)
- Sxf `logic [((4'bx100 ==? 4'b1?00) & 1'b0) : 0] v;` iv/sv/vl bits 1; PRE + POST-T bits 1; POST E3009. Sxg `? 0 : 0` same. Sxk `inside` + `& 1'b0`: vl bits 1, PRE/POST-T bits 1, POST E3009.
- control Sxl `((4'bx100 != 4'b1100) & 1'b0)` (no wildcard): bits 1 on all three vita (old route kept).
- wrong->loud siblings: Sxc `? 3 : 3` (oracles 4/4; PRE 1/x; POST loud), Sxj `| 1'b1` (oracles 2; PRE 1; POST loud), Sxe dim `[(..&1'b0)+1]` (oracles size 1; PRE x; POST loud). Fixed: Sxa `&& 1'b0` (dim x->1), Sxb `|| 1'b1` (1->4), Sxh `=== 1'bx` (1->2).
- PRE's right answer is the bound catch-all's one bit coinciding with [0:0] (accidental). Root of the decline = documented residue X05/X19/X20 (4-state & | ?: with x in the bit domain).
- position count for bound `[((4'bx100 ==? 4'b1?00) & 1'b0) : 0]` (value 0, oracles 1 bit): value->loud on PRE-right = 6 (Sxf var, Sxg `?0:0`, Sxk inside, Sy2 port, Sy3 wire, Sy5 function return); wrong->loud = 5 (Sy1 parameter-type range: PRE 32 bits; Sy6 unpacked [X:0]: PRE 8; Sxc `?3:3` 1 vs 4; Sxj `|1'b1` 1 vs 2; Sxe dim x vs 1); loud->loud Sy4 typedef (PRE E3010 other cause); unaffected Sy7 `V[X +: 2]`; unchanged loud Sy8 bit-select/replication. All POST-T = PRE => half S.
- T31: label lane reads S-bound binders (package / imported / package 65-bit / generate-scope 65-bit !=? / loop-scope 65-bit genvar ==? / #() override P and 65-bit Q): POST = sv2v on all 13 lines; PRE + POST-T E3009.
- T32b/T33 PRE-EXISTING (not this branch): `localparam T TP = '1;` with `#(.T(logic signed [3:0]))` or `#(.T(byte))` reads 15 / 255 unsigned in every consumer ($display, `TP<0`, generate-if, procedural case, generate-case); iv/sv/vl -1. POST = POST-T = PRE. Not found in ROADMAP by grep (`T$s`, `localparam T `): UNVERIFIED whether queued. u_iu (`int unsigned`) moved def->hit = oracles.
- Sz1: wildcard x in an untaken ?: arm / after `1'b0 &&` / `1'b1 ||`: folds (no refusal); `||` bound 1->2 bits = oracles.
## Claims
1 CONFIRMED (vita side re-run 252/252; oracle columns inherited) + random sweeps: 487 moved cases = iv AND vl.
2 CONFIRMED in substance: 31/34 inherited oracle-split cells keep PRE; moved 3 = N09/NA11G/NA11S rulings. The number 24 NOT MEASURED (needs the pair/whole instrument).
3 NOT MEASURED directly (no instrument in the frozen binaries); indirect: shadow/binder probes T03 T04 T23 T28 T31 T32b show no name read differently.
4 CONFIRMED on T24 (nets $bits, processes, outside `assign g.w` agree), T26b, T27b.
5 REFUTED on value->loud: >= 6 more value->loud cells whose bound value is defined (Sxf Sxg Sxk Sy2 Sy3 Sy5; oracles agree; PRE right). loud->value part supported: 91 new loud->value cells, all = oracle.
6 CONFIRMED (cmp4 re-run identical; all 58 non-REF variants are generate-case designs).
7 CONFIRMED for -G 65-bit (T02), defparam (T16), override chain (T17), loop-scope 65-bit (T05, T27b), generate-scope 65-bit (T26b, T31), package/imported/override ==?/inside (T31).
8 NOT MEASURED.
## Counts
hand-written designs ~49 (T01-T33 incl. hand-edited b/p/v/g twins, S01-S04 S02b S02c, gchk); template variants 22 (Sxa-m, Sy1-8, Sz1); script cells: Sq/Si 92, Sp/Sg 28, random generate-case files 127 = 3099 cases.
