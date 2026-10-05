# lens:differential round 1 — s582 case-inside
status: IN PROGRESS
PRE=S/pre/vita md5=4ace7617440041d0d16aee5309fded10
POST=S/post/vita md5=f285440f37b22c9cdf32eeb8ebb1e721

## Questions / findings (updated per question)

### F1 BLOCKING — identifier `inside` as the first token of a case item (Verilog-2005 / `begin_keywords "1364-2005"`): PRE correct, POST silent-wrong
- parser: `stmt_ctl.rs` `if self.at_ident_kw("inside") { return self.parse_case_inside(...) }` — contextual word, no keyword-set check; vita has no `begin_keywords` / .v keyword mode (grep: none).
- d/d01_ident.v (`reg [3:0] inside=4'b0110`; `case (x) inside[2:1]:`, `inside + 1:`, `inside & 4'hF:`):
  PRE `A m1=1 B m2=1 C m3=1` rc=0 | POST `A m1=0 B m2=0 C m3=0` rc=0 (no diagnostic) | iverilog -g2005 `A m1=1 B m2=1 C m3=1` | verilator --default-language 1364-2005 `A m1=1 B m2=1 C m3=1`
- d/d03_bk.sv (same body under `` `begin_keywords "1364-2005" ``, .sv): PRE 1/1/1 | POST 0/0/0 rc=0 | iverilog -g2012 1/1/1 | sv2v->iverilog 1/1/1 | verilator 1/1/1
- d/d02_ident_loud.v (`case (x) inside: …`, `case (x) inside, 4'd9: …`): PRE `D m1=1 E m2=1` rc=0 | POST `error[VITA-E2002] … expected expression, found ':'` + `found ','` rc=1 | iverilog -g2005 `D m1=1 E m2=1` | verilator `D m1=1 E m2=1`  → loud regression
- 4-way: real gap (POST regression; PRE = 3/4 oracles). Census m9 had the same shape but both printed m=0 by coincidence and was filed "SV-illegal, no oracle".

### Q1a literal/sizing probes (accepted set) — no slice finding
- d/d05_constop.sv (const-folded operator items `~4'b0011`, `4'd15+4'd1`, `A+4'd1`, `[4'd15+4'd1:8'h10]`, `?:`, `<<`, `-4'd1` vs 8-bit e): POST `m1..m8=1` = sv2v->iverilog = verilator. PRE E2002 x11. clean.
- d/d06_signed_xmsb.sv (signed e vs `4'sbx011`/`4'sb?011`, sign-extended wildcard): POST `m1=1 m2=1 m3=1 m4=0` = sv2v->iverilog; verilator `m1=0 m2=0 m3=0 m4=0` (x/z: not an oracle). clean (no-oracle on verilator side).
- d/d04_unsized_xz.sv (64-bit e vs unsized `'b?1`,`'hx`,`'bz1`): POST `m1=1 m2=1 m3=1 m4=1 m5=0 m6=0 m7=1`; sv2v->iverilog `m1=0 m2=0 m3=0 m4=0 m5=1 m6=0 m7=0`; verilator `m1=1 m2=1 m3=1 m4=1 m5=1 m6=0 m7=1`. Case-inside cells (m1-m3): POST = verilator = §5.7.1 (unsized x/z MSB extends to the expression width); sv2v->iverilog contradicts itself (casez m5=1 extends, ==? m7=0 does not) -> disqualified. m5 (plain casez) POST=0 vs both oracles 1 -> pre-existing candidate, see PE1.

### PE1 pre-existing (not slice code) — plain casez/casex do not extend an unsized x/z-MSB literal past 32 bits
- d/d07_casez_unsized.sv (64-bit e=64'hFFFF_FFFF_0000_0001; `casez (e) 'b?1`, `casex (e) 'bx1`): PRE `casez m1=0 casex m2=0 inside m3=1 wildeq m4=1` | POST identical | sv2v->iverilog `casez m1=1 casex m2=1 inside m3=0 wildeq m4=0` | verilator `casez m1=1 casex m2=1 inside m3=1 wildeq m4=1`. Both oracles: casez/casex = 1 (§5.7.1). PRE=POST=0. Not touched by the slice; vita's `inside`/`==?`/case-inside extend (=verilator), its casez/casex do not.

### Q6 verilator `inside`-set sizing — MEASURED: collective (widest element), not per pair
- d/d08_q6_sizing.sv a4=4'hF,b4=4'h1: `(a4+b4) inside {4'h0,8'hFF}` / `{8'h10}` / `{4'h0}` / `{[4'h0:4'h0],8'hFF}` / `{8'hFF,4'h0}`
  verilator `q6 r1=0 r2=1 r3=1 r4=0 r5=0` (collective) | sv2v->iverilog `r1=1 r2=1 r3=1 r4=1 r5=1` (per pair) | PRE = POST `r1=1 … r5=1` (per pair).
  => PROBE_CATALOG ~87 ("verilator sizes an inside set by its widest element") is right for an operator lhs; per-pair answers only on single-width sets (r2, r3).

### Q2/Q3 context + unique probes — no slice finding
- d/d11_ctx.sv (always @* operator e + [lo:hi] var bounds, generate-for genvar bounds, fork, automatic recursion, final): POST = sv2v->iverilog = verilator on all 7 lines.
- d/d09_unique.sv: POST W4031 on U1ci/U1pl/P1ci/P1pl (4 = 2 inside + 2 plain twins), none on unique0, user default (UDci m=9) and overlap (UOci/UOpl m=1, both silent: overlap unchecked in both); values = sv2v. verilator --binary asserts+$stop at first violation (not comparable).
- d/d10a_pop.sv: POST E3009 "a queue pop is only supported as the DIRECT rhs…" on both case-inside and plain twins (loud, pre-existing limit); verilator `ci pop m1=3 size=5`. d/d10b_inc.sv: `case (cnt++)` E2002 on PRE and POST (parser), oracles `m3=5 cnt=6`.

### Q1b accepted-set probes (batch 3)
- d/d15_sign.sv (`$signed(v8)` e, `8'(4'sb1111)`, `signed'(8'hFF)`, signed enum labels, signed packed struct e vs 16-bit signed value/range): POST `m1=1 m2=1 m3=1 m4=1 m5=1 m6=1` = sv2v->iverilog = verilator. clean.
- d/d14_wide.sv: POST refuses the whole design on `case (e33) inside 'd2 ** 32:` (E3009 clause 5 "x/z bits known only at run time") where sv2v = verilator = `m1=1`; over-refusal of a constant `**` item (loud, not a shake). Rest re-run in d14b.
- d/d12_itemcall.sv: POST loud (pre-existing E3009 "function `g` assigns a module net…"); sv2v duplicates item calls (`g(1)` x7), verilator evaluates twice — no order oracle (as grounding m3).
- d/d13a_arr.sv / d13b_q.sv (unpacked array / queue / dynamic array as a case-inside item): POST loud E3009 ("a whole unpacked array has no value…", "a dynamic-storage handle has no whole-value surface…"); verilator C++ compile error; sv2v->iverilog `m1=0 m2=0 m3=0` (no array expansion); no oracle, POST loud.
- d/d14b_wide.sv (33/65/128-bit unsigned e vs boundary-carry operators; signed 33/65/128-bit e vs 32-bit signed ranges): POST `m2=1 m3=1 m4=1 m5=1 m6=1 m7=2 m8=1 n1=1 n2=1 n3=1 n4=0` = sv2v->iverilog; verilator differs only n2=0 (`s65=-1 in [-2:0]`, verilator narrow-signed-range class P4). clean.
- d/d16_members.sv: POST E2002 (signed-typedef packed-array element unsupported) — whole design loud; re-run without it in d16b.

### Q5 backends / staged / VCD — no finding
- 9 accepted designs (d04 d05 d06 d09 d11 d14b d15 d01 d03) x {default, --backend interp, vm, native, staged vcmp->velab->vrun on S/post/vita}: stdout md5 identical per design (e.g. d11 ec90ddd9 x5, d01 dabd6953 x5 — F1's wrong value reproduces in every lane).
- d11 VCD ($dumpvars(0,top) around 4 case-inside sites): 1277 bytes, byte-identical across default/interp/vm/native/staged; no `$var` containing tmp/$ia/case.
- d/d16b_misc.sv: POST E3018 on `assign y` to `int y` (pre-existing limit) — rerun as d16c. Oracles: `m1=1 m2=1 m5=1 m6=1 w1=2 w2=2 w3=0 w4=1 o1=2 o2=0`.
- d/d17_clsrand.sv (class method `case ($urandom_range(0,1)) inside`, `case ($random & 1) inside`): POST refuses both (clause 6 "cannot be evaluated exactly once", and "a call returning a signed type") besides pre-existing E3010 class for-var errors; d17p plain twin: PRE = POST (same E3010s). verilator `c=0` / `c=55` (verilator re-evaluates $random in plain case too). Guard holds.
- d/d16c_misc.sv (signed struct member `s.a`, signed unpacked element, wildcard item WIDER than e unsigned/signed, case-inside in a function feeding `assign`): POST `m1=1 m2=1 m5=1 m6=1 w1=2 w2=2 w3=0 w4=1 o1=2 o2=0` = sv2v->iverilog = verilator. clean.
- d/d18_opsign.sv: POST refuses `$signed(u4) + 1` item (clause 6: `$signed` counts as a call) -> whole design loud; verilator rejects `e8 inside {s4+1}` ("RHS of ==? is fourstate but not a constant"). d/d18b_opsign.sv (signed 2-state operand inside an operator item under an unsigned pair: `s4 + 1`, `[s4+1:8'd20]`, `(s4+0) >>> 1`): POST `m1=1 m2=1 m4=1 m5=1 m9=1 m9q=1` = sv2v->iverilog = verilator. clean (context sign reaches operands).
- d/d19_xhoist.sv (x in a hoisted operator/call e: `a|b`, `fx(a)`, `a+4'd0`, `a^b` vs range, frame-capture in a function): POST `m1=2 m2=2 m3=2 m4=0 m5=2` = sv2v->iverilog; verilator `1 1 2 1 1` (2-state, not an x oracle). clean.
- case_ctx_operator (case_inside.rs:25-54) is token-for-token the list removed from case_operand_takes_ctx (stmt_flow.rs hunk @@-599): plain-case §12.5 classification unchanged.
- d/d20_twocomb.sv (two `always @*` case-inside on hoisted operator e + two plain twins; x e vs `1'b?`): POST (perl alarm 30, rc=0) = sv2v->iverilog = verilator: `t1 m1=1 m2=2 n1=1 n2=2`, `t2 m1=2 m2=1 n1=2 n2=1`, `w=1`. clean.
- F1 extension, d/d21_ident2.v (.v: `casez (x) inside[2:1]:` with `reg inside`; plus `case (x) inside(3):` with `function inside`): PRE `G m2=1` `F m1=1` rc=0 = iverilog -g2005 = verilator 1364-2005; POST `error[VITA-E2002] … expected `case` before `inside` — `casez` / `casex` have no `inside` form …, found identifier 'inside'` rc=1 (loud regression, same root as F1). The `inside(3)` function-call item could not be observed separately (the casez parse error stops the run) -> UNVERIFIED as a silent instance.
- gate glance: S/cli2.log `Summary [22.842s] 7961 tests run: 7961 passed, 1 skipped`.
- d/d22_castleak.sv (size cast / concatenation as an item under a wider e: `8'(a8+b8)`, `{a8+b8}`, `[8'(a8+b8):8'(a8+b8)]`, `(a8+b8)`; a8=FF b8=01 e16=0100): POST `m1=0 m1q=0 m2=0 m2q=0 m3=0 m4=1` (default/interp/vm identical) = verilator = §6.24.1; sv2v->iverilog `m1=1 m1q=1 m2=0 m2q=0 m3=1 m4=1` (sv2v drops the size-cast truncation in both the case-inside and the `==` twin -> disqualified on size casts). clean.

### F1 contract note
- docs/manual/003_language-reference.md:203: "`begin_keywords "spec"` / `end_keywords` | Ignored | One logical line is consumed; the full keyword set applies regardless." Under the full 1800-2017 set `inside` is reserved, yet PRE and POST both accept `reg [3:0] inside;` with no diagnostic; POST then reinterprets the use. Either reading (legal Verilog-2005 input, or illegal input accepted silently) ends in a silent value change vs PRE and vs every tool that accepts the input.

## Verdict
verdict: FINDINGS — product_shakes: yes (F1: identifier `inside` leading a case item now parses as `case … inside`; PRE-correct values change with no diagnostic, and two shapes go from running to E2002).
Accepted-set boundary (Q1), evaluate-once/contexts (Q2), unique/priority (Q3), backends/staged/VCD (Q5): no slice finding in 30 designs.
Over-refusals seen (loud, PRE also loud, NON-BLOCKING): `'d2 ** 32` item (clause 5), `$signed(u4)+1` item (clause 6 counts `$signed` as a call).
status: DONE

# Round 2 — delta review (POST2)
status: IN PROGRESS
POST2=S/post2/vita md5=f491ab7b4921b912baa27b9828641232 (expected)

## R2-Q1 re-score (all 29 round-1 designs + census v01 v05 v07 v12 x01; POST vs POST2, stdout+stderr+rc byte compare, VCD where written)
- SAME 30 / CHANGED 4. Changed = exactly the 4 designs that declare `inside`: d01_ident.v, d03_bk.sv (POST `A m1=0 B m2=0 C m3=0` rc=0 -> POST2 E3009 at the case keyword, x3), d02_ident_loud.v (POST E2002 x2 rc=1 -> POST2 `D m1=1 E m2=1` = PRE), d21_ident2.v (POST E2002 casez -> POST2 casez parses, `case (x) inside(3):` E3009 at 7:5).
- Every design with no `inside` declaration: byte-identical (d11 VCD identical too). No finding.

## R2-Q2a Verilog-2005 declaration triage (r2/d/r2a.v; r2a_c.v = r2a.v + a control module `ctl` with a genuine case-inside and no `inside` declaration)
- 15 shapes: function formal, function local, automatic recursive function local, function named-block local, task formal, task local, `#(parameter inside)`, localparam, specparam, genvar (generate-for), generate-if block reg, enclosing named block reg, implicit net (`assign inside = 1'b1;`), use-before-declare, input port.
- POST2: E3009 "a name `inside` is declared here…" at every one of the 15 case lines (6,17,29,40,49,60,67,72,77,83,90,99,106,110,116); the control's line 128 has no error (no cross-module contamination). 0 misses.
- verilator --default-language 1364-2005: `ff=1 fl=1 fa=1 fn=1 tf=1 tl=1 p=1 lp=1 sp=1 gv=1 gb=1 nb=1 im=1 ubd=1 port=1`; iverilog -g2005 rejects use-before-declare (`Unable to bind wire/reg/memory 'inside' in 'top.u14'`); PRE rejects it too (E3010 "`inside` is used before it is declared"), so PRE stopped before printing values.

## R2-F1 BLOCKING (same root class as round-1 F1) — upward-resolved function `inside`: PRE loud, POST2 silent
- r2/d/r2d_up.v (Verilog-2005): `function [3:0] inside; input [3:0] v; inside = v + 4'd1; endfunction` in parent `top`; child `case (x) inside(3): m = 1; default: m = 0; endcase` with x=4.
  PRE `r2d_up.v:4:27: error[VITA-E3010] E-ELAB-UNRESOLVED-NAME: call to undeclared function `inside` [in top.u]` rc=1 | POST2 `up m=0` rc=0 errors=0 | iverilog -g2005 `up m=1` | verilator --default-language 1364-2005 `%Error: r2d_up.v:4:36: Can't find definition of task/function: 'inside'` (rejects -> not evidence).
  name_inside_binds asks only the current scope's `has_func`; the plain reading binds by upward task/function name search (iverilog). Loud -> silent.
- r2/d/r2f_unitfn.sv ($unit `function inside`, `case (x) inside(3):`): PRE `unitfn m=1` rc=0 | POST2 E3009 name reason (caught, loud).
- r2/d/r2e_unitvar.sv ($unit `logic [3:0] inside`): PRE and POST2 E2002 "expected 'module', found keyword 'logic'" (vita has no $unit variables) — unreachable.
- r2/d/r2b.sv: PRE and POST2 both E2002 at 17:3 (class `localparam` unsupported) — rerun without it as r2b2.
- r2d_up.v POST2 on every lane: interp/vm/native/staged vcmp->velab->vrun all `up m=0` (velab `errors=0`).

## R2-Q2b SV-only declaration triage (r2/d/r2b2.sv = r2b.sv without the class-localparam class, which PRE and POST2 both reject at parse)
- case lines: 10 class property, 14 class method `inside(3)`, 18 inherited property, 21 method formal, 24 method local, 29 enum label, 34 `let inside`, 39 `import pk::*`, 44 `import pk::inside`, 49 `import pf::*` fn, 54 `import pf::inside` fn, 58 `for (int inside = 6; …)`, 62 `foreach (arr[inside])`, 67 `for (genvar inside …)`, 84 control (no declaration).
- POST2 E3009 (name reason) at 10 14 18 21 24 29 34 39 44 49 54 67 — MISSING at 58 (for-loop variable) and 62 (foreach index); none at 84 (control OK).
- PRE: only errors are in the control (`undeclared net/variable ctl.inside` + part-select), i.e. PRE elaborated lines 58 and 62 with the plain-label reading.

## R2-F2 BLOCKING (same root class as round-1 F1; no oracle) — for-loop variable / foreach index named `inside` is not seen by name_inside_binds
- r2/d/r2h_loopvar.sv: `for (int inside = 6; inside < 7; inside++) case (x) inside + 1:` (x=7), `foreach (arr[inside]) case (x) inside + 1:` (int arr[7], x=7), `for (int inside = 6; …) case (x) inside[2:1]:` (x=3).
  PRE `forvar m=1` `foreach m=1` `forvar2 m=1` rc=0 | POST2 `forvar m=0` `foreach m=0` `forvar2 m=0` rc=0 errors=0 (interp/vm/native identical) | iverilog/sv2v/verilator: no oracle (IEEE 1800 reserves `inside`; `for (int …)` and `foreach` are SV-only, so no 1364 framing).
- r2/d/r2i_blocks.sv: unnamed-block local (line 4) and fork-local (line 8) caught (E3009); `for (int inside …)` inside an automatic function (line 14) NOT caught (no error at 14); PRE `ublk m=1 fork m=1 ffor m=1`. Its POST2 value is hidden by the two caught lines -> value UNVERIFIED, acceptance measured.

## R2-N1 NON-BLOCKING (accepted-residue class of t/f1f; new token instance `.`) — instance named `inside`, hierarchical item `inside.v` (Verilog-2005 legal)
- r2/d/r2c_hier.v: PRE `hier m=1` rc=0 | POST2 `r2c_hier.v:6:45: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected expression, found '.'` (x3) rc=1 | iverilog -g2005 `hier m=1` | verilator 1364-2005 `hier m=1`. Loud regression vs PRE-correct with 2 oracles; same parser-rule cost as the accepted `inside == 4'd5` (f1f).
## R2-N2 NON-BLOCKING (same residue class; SV-illegal, no oracle) — typedef named `inside`, cast item `inside'(8'h15)`
- r2/d/r2k_tdcast.sv: PRE `tdcast m=1` rc=0 | POST2 `r2k_tdcast.sv:5:42: error[VITA-E2002] … expected expression, found '''` rc=1. Loud.
- R2-F2 extension: `--top m_ffor` on r2i_blocks.sv (for-loop variable `inside` inside an automatic function): PRE `ffor m=1` rc=0 | POST2 `ffor m=0` rc=0 errors=0 -> silent, confirmed.

## Round 2 verdict
verdict: FINDINGS — product_shakes: yes. name_inside_binds misses (a) a function `inside` reached by upward task/function name search (Verilog-2005 legal; PRE loud E3010, POST2 silent `m=0`, iverilog `m=1`) and (b) SV `for (int inside …)` / `foreach (arr[inside])` loop variables, in modules and in functions (PRE plain-label 1, POST2 silent 0; no oracle).
Caught (E3009, loud): 15 Verilog-2005 shapes (r2a) + 12 SV shapes (r2b2: class property/method/inherited/formal/local, enum label, let, pk::* / pk::inside params, pf::* / pf::inside functions, inline genvar) + generate-block function, functions called from assign / always @* (r2g), $unit function (r2f), unnamed-block and fork locals (r2i). Control modules with no declaration: not refused.
Re-score: 30/34 byte-identical; the 4 changes are exactly the designs that declare `inside`.
status: DONE

# Round 3 — delta review (POST3, last round)
status: IN PROGRESS
POST3=S/post3/vita md5=9b0c1623e3acb71130efe3acb7713a89 (expected); compared against POST2 md5=f491ab7b4921b912baa27b9828641232

## R3-Q1 re-score (29 round-1 + 12 round-2 designs + census v01 v05 v07 v12 x01 + r2i `--top m_ffor`; POST2 vs POST3 byte compare)
- SAME 36, CHANGED 11: d01, d03_bk, d21, r2b2, r2f, r2h, r2i, r2a, r2a_c, r2d_up, r2g — every one uses `inside` as a name. POST3 rc=1 with E3009 on all 11. silent->loud: r2h (POST2 `forvar m=0 foreach m=0 forvar2 m=0` rc=0), r2d_up (POST2 `up m=0` rc=0), r2i --top m_ffor (POST2 `ffor m=0` rc=0); the rest loud->loud (reason text changes). loud->silent: none.
- Unchanged name-use designs: d02 (plain labels `inside:` / `inside, 4'd9:`, `D m1=1 E m2=1`), r2c_hier / r2k_tdcast (E2002), r2e / r2b (E2002). All designs without a name `inside`: byte-identical (d11 VCD identical); census v01 v05 v07 v12 x01 identical (= R3-Q3 spot check).

## R3-Q2 attack — preprocessor / attribute / order (single file)
- `define NM inside (r3a), token paste a``b -> ins``ide (r3b), macro argument (r3c), `-D NM=inside` on the command line only (r3d), `include of a file declaring `inside` (r3e): POST3 E3009 on every one, "first at" naming the right file:line (r3e: r3e_decl.vh:1). PRE `m=1` on all; iverilog/verilator `m=1` on r3a r3b r3c r3d (r3e: oracles run from another cwd could not find the include — harness-format). Count is on the post-preprocess tokens.
- string + comments + unused macro body (r3h1): POST3 runs `inside m=1` (not counted). Attribute `(* inside *)` (r3h2): POST3 runs `attr m=1` (not counted; an attribute binds nothing). Name use only AFTER the case-inside, inside a generate block (r3i): POST3 E3009 (first at :6). No finding.
- r3e oracles rerun with -I: iverilog -g2005 `include m=1`, verilator `include m=1` (= PRE; POST3 refuses).

## R3-Q2 attack — several files, -f, library compose
- Upward function across files (r3f_top.v defines `function inside`, r3f_child.v has `case (x) inside(3):`): one-shot child,top / top,child / `-f` list: POST3 E3009 (first at r3f_top.v:3); PRE E3010 "call to undeclared function `inside`"; iverilog -g2005 `up2 m=1`. Library compose (vcmp --work, same lib; separate libs T,C and C,T; velab -L --top top): POST3 E3009 on all three ("first at byte 30"); PRE E3010.
- Package across files (r3g_pkg.sv localparam `inside`, r3g_mod.sv `import p::*; case (x) inside + 1:`): one-shot both orders POST3 E3009, PRE `pkg m=1`; one `vcmp --work` with both files and `vcmp -o .vu` + velab: POST3 E3009, PRE `pkg m=1`; two vcmp into one or two libraries: PRE and POST3 both `import from unknown package p` (pre-existing library limit, loud).
- $unit function in its own file (r3j_fn.sv) + module (r3j_mod.sv): one-shot POST3 E3009 (first at r3j_fn.sv:1), PRE `unitfn2 m=1`; one vcmp with both files: POST3 E3009, PRE `unitfn2 m=1`.

## R3-N1 NON-BLOCKING (new instance; not F1's root) — vcmp --work now accepts a source with no design units when it uses `inside` as a name
- `vcmp --work J=dir r3j_fn.sv` (file = one $unit `function … inside(…)` only): PRE `error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: no design units found in source` rc=1, no library | POST3 rc=0, `warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT …`, lib.toml + units written. Control r3k_fn.sv (same shape, function `ff`): PRE and POST3 both E2002. Root: `TopItem::InsideNameUse` makes the unit non-empty for vcmp's emptiness check.
- Consequence measured: `velab -L K -L J --top top` + vrun runs r3j_mod's case-inside: POST3 `unitfn2 m=0` rc=0 (PRE cannot build J). Separate vcmp runs are separate compilation units, so J's $unit function does not bind in K; m=0 is the case-inside answer, not a wrong value. One-shot `vita r3j_fn.sv` alone stays loud (PRE E2002 -> POST3 `E3009 no top module to elaborate`).
## R3-N2 NON-BLOCKING cosmetic — library-mode refusal names a byte offset: "(first at byte 30)" / "(first at byte 36)" instead of file:line (no span resolver in velab -L / .vu mode).

## Round 3 verdict
verdict: FINDINGS (2 NON-BLOCKING) — product_shakes: no. No silent case-inside over a name use was found: macro / paste / macro-arg / -D / `include / generate / after-position / multi-file / -f / library compose (named units) / .vu all refuse; strings, comments, unused macro bodies and attributes do not count (no over-refusal). Re-score: 11 changes, all name-use designs, all end loud (3 silent->loud), 0 loud->silent; 36 byte-identical incl. 5 census cells and d11's VCD.
status: DONE
