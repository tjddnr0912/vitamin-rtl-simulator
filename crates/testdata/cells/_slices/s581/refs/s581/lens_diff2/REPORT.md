# lens:differential round 2 — REPORT (final)

task: lens:differential round 2 · verdict: FINDINGS (2 BLOCKING) · term: completed (36 tool calls, 24 designs + 29 variants)
binaries (md5 verified): PRE 4ace7617440041d0d16aee5309fded10 · POST 1e41978f78724cc11b691cf6babfe911 · POST2 ed69d5ae6fa11e12d4220550f6391b42 · POST-T 37d2b613f937979ade730b6808175e22 (attribution only)
harness: run2.py (= round-1 lens run.py; tools iv/sv/vl/pre/post/post2); cells p/<id>.sv, raw p/<id>.<tool>.txt; batch outputs run_all.out run_b2..b6.out
claim harnesses: moves.py (census/s2/lens dirs), h666.py (+h666.out), velab/ (corpus_velab.py copy; pre/post2 .velab/.vu)

## F-A BLOCKING — fix 1 (stale-entry skip), NEW root introduced by 9bd5cd65
An explicit import of a WIDE package constant after a wildcard import whose package has a NARROW constant of the same name: the key holds both entries and the WIDE one is current (LRM 26.3: a wildcard candidate is not imported when the name is explicitly imported). `wide_entry_is_stale` treats any key with both as narrow-current.
Mechanism (POST2 source): package.rs:1204-1234 wildcard binds pa's narrow const into `params` at fq(name) (`explicit_imports` empty at that point); package.rs:~1373-1386 explicit import inserts `wide_param_bits` at the same key and never unbinds `params` -> const_wide.rs `wide_name_bits` declines, ident_route.rs `bare_ident_route` skips Wide.
- p/q1g.sv `import pa::*; import pb::P;` (pa::P=3, pb::P=65'h1_0000_0000_0000_0009)
  iv   P=18446744073709551625 Q=18446744073709551626 w=18446744073709551625 sh=16 N=5 b=65 | big | hitw
  sv   (same as iv)
  vl   P=3 Q=4 w=3 sh=0 N=5 b=32 | hit3            <- disqualified: q1g4 (same imports, explicit first) vl reads 18446744073709551625 | big | hitw: order-dependent
  PRE  P=18446744073709551625 Q=18446744073709551626 w=18446744073709551625 sh=16 N=5 b=65 | hit3
  POST P=18446744073709551625 Q=18446744073709551626 w=18446744073709551625 sh=16 N=5 b=65 | hitw
  POST2 P=3 Q=4 w=3 sh=0 N=5 b=65 | hit3
- p/q1g2.sv (adds port, part-select, procedural): PRE/POST `P=...625 Q=...626 w=...625 ps=16 x=18446744073709551627 b=65 | port pi=18446744073709551625`; POST2 `P=3 Q=4 w=3 ps=0 x=5 N=5 b=65 | port pi=3` (iv/sv reject the design's illegal `top.P` hier read; vl P=3)
- p/q1n.sv package constants only (`package pk; import pa::*; import pb::P; localparam [64:0] Q = P + 65'd1; R = P;`):
  iv/sv `Q=18446744073709551626 R=18446744073709551625 M=5` · vl `Q=4 R=3` · PRE/POST = iv · POST2 `Q=4 R=3 M=5`
- 4-way: real gap (right->wrong vs PRE at run time and in constant folds). Not reached: header param / override / body localparam + wildcard (q1q, q1fv), real/string wildcard candidates (q1p), explicit-first order (q1g3/q1g4).

## F-B BLOCKING — fix 2 incomplete; same root as round-1 F2 (phase-dependent region), NEW instance; introduced by T (POST-T mixes too)
The region is AVAILABLE in the Nets phase too, so `gen_case_region` caches true, but a forward label resolves to an OUTER same-named constant in Nets and to the inner generate-scope localparam later; the region decision flips, the i64 decision does not.
- p/r1.sv outer `K=8'hFF`, inner forward `K=32'hFFFFFFFF`, `case (-1) K: (8-bit w=200) default: (4-bit w=9)`
  iv def 9 bits=4 · sv k 200 bits=8 · vl k 200 bits=8 · PRE def 9 bits=4 · POST-T k 8 bits=4 · POST k 8 bits=4 · POST2 k 8 bits=4  (mix: default's 4-bit net, arm k's init and process)
- p/r1b.sv outer `K=32'hFFFFFFFF`, inner forward `K=8'hFF`
  iv k 200 bits=8 · sv def 9 bits=4 · vl def 9 bits=4 · PRE def 9 bits=4 · POST-T def 9 bits=8 · POST def 9 bits=8 · POST2 def 9 bits=8  (mix: arm k's 8-bit net, default's init/process)
- control p/r1p.sv (i64 decision flips: outer K=5, inner K=6, case (8'd5)): PRE/POST/POST2 all `def 9 bits=8` (pre-existing d1p class).
- 4-way: real gap (a mix matches no oracle; r1b: PRE = sv2v+vl -> POST2 wrong).

## Residues / pre-existing (not blocking: POST2 = PRE)
- fix 3, NEW instances of recorded classes: x-valued bounds silent on POST2 = PRE = iv `1` (sv2v `x`): `~(x==?)`, `(x==?)^1'b1` (4-state op class), `+1'b0`, `*2`, unary `-` (arithmetic class), `?1:0` (?: class) — q3x2. Value-defined `((x==?) ? 1'b1 : 1'b1)`: PRE 1 / POST loud / POST2 1, oracles 2 (Sxc class) — q3m3. Replication count `{B{1'b1}}` with x-valued B: silent on POST and POST2 (PRE rep=0) — sF3 class — q3r_x2 line 18.
- q1g generate-if `if (P > 65'd100)` false on PRE/POST/POST2 (iv/sv `big`); q1g gen-case PRE hit3, POST hitw, POST2 hit3.
- loud on all builds where oracles agree: q1cv (string default overridden by 65-bit), q1o (generate-block import), q1r (65-bit enum labels), r6 (generate in interface), q1f/h/j/k/l/m original (65-bit gen-case scrutinee E3010).

## Clean
- fix 1: override (q1av), real default overridden (q1bv), -G (q1dv), defparam (q1ev), wildcard narrow + body wide localparam (q1fv), const-fn formal/local (q1hv), $unit (q1kv), per-instance overrides + hier reads (q1lv), package fn local (q1mv), header param/override/header import + wildcard (q1q), gen-scope wide beside genvar (q1i): POST2 = oracles. q1jv in-loop `$display(i)` moves to the oracles (A1D class), after-loop wide reads stay right.
- fix 2: macro-expanded (r4) and `include`d (r5) twin cases in one scope: no span collision. Named/unnamed loops (r2/r3, r7/r7u), nested gen-case in per-instance gen-if arms (r6b): no mix.
- fix 3: q3m2 (11 shapes), q3i (8 inside/combined shapes), q3r_m2 (11 routes): POST2 = oracles, zero refusals. q3r_x2 x-valued bound refused on typedef, interface signal, class member, port, packed, localparam range, function return, unpacked; `B*3` back to PRE catch-all; part-select width and replication silent on all builds.

## Claims re-measured
- 0/252 census POST2!=POST: CONFIRMED (moves.py, raw bytes).
- 0/174 matrix: CONFIRMED at verdict level; raw bytes 2 (AD_X, RB_X: refusal wording only, loud on both).
- 14/666 harness: CONFIRMED (h666.out): S05A/B S07A/B K02q -> PRE; S08A/B S14A/B K02 loud->loud; Q3 -DFNRX/-DTDX (+-DIV) loud->value.
- 32/410 lens probes: CONFIRMED (321 lens_diff/p: 12, all = PRE; 89 lens_snd/probes: 20, 18 = PRE, A1D/B2D != PRE).
- "every move to PRE's answer except A1D/B2D": REFUTED (cell) literally for the 666 harness: S08A/B S14A/B K02 stay loud (PRE values); Q3 FNRX/TDX: PRE loud on unrelated lines, so POST2's value is not PRE's answer. Neither is a regression.
- .velab/.vu PRE vs POST2: CONFIRMED 15/15 + 15/15 byte-identical (velab/).
