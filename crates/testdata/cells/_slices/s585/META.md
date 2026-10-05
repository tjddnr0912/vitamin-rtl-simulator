# META s585 (phase 1 manifest; nothing copied)

Status: COMPLETE. Files: MANIFEST.tsv, CELLS.tsv, EXCLUDED.tsv (this dir). Helpers kept here: inv.txt (+.xdirs)
= file inventory with md5, outs.json, build585.py, rules585.py, excl.py, size.py, size.txt.

## Slice
- §4.5.585. ROADMAP row: "§4.5.585 — ROADMAP §5.2 row 1, §3.b unique-if-chain" (s585/COMMIT_SLICE.txt:3).
- Outcome (slice commit title): "parser: a `unique if … else if` chain in procedural code reports its no-match
  (§3.b unique-if-chain)" (s585/COMMIT_SLICE.txt:1). Docs commit: "`unique-if-chain`'s residue is BLOCKED on new
  §2 🆕 AB and `unique-pkg-closure`; 🆕 AB is §5.2 row 3" (s585/COMMIT_DOCS.txt:1).
- Slice start: $S/s585_start = 1790981056 = 2026-10-03 07:44:16 KST (first harness run g/q1a.log 07:50).

## Binaries
| tag | path under $S | md5 | profile | base commit / branch | md5 source |
|---|---|---|---|---|---|
| PRE | s585/pre/vita | c766b8d59edabaef96436e7b46e75dc6 | release | 75f255d4 (s585/pre/HEAD) | s585/GROUNDING.md:3 (= md5_at_preservation) |
| PRE-H | PRE binary run on the cell's `_H` spelling (aux) | same as PRE | release | 75f255d4 | s585/PLAN.md:120 ("H spelling = PRE's armed tree") |
| POSTC | s585/postc/vita (POST-cand) | 17b7d988696c0f5b38b7e94b845e2df9 | release | wt-s585 = 75f255d4 + s585/postc/cand.patch | s585/GROUNDING.md:166 (= md5_at_preservation) |
| POST | s585/post/vita (POST-c2, review round 1) | 6328a971c02b99269f42a1b39f545fda | release | wt-s585 + slice.patch md5 ce8e4ae7879e48cbc652a6ac1fcdd818 | s585/r1/sound/REPORT.md:4 (= md5_at_preservation) |
| POST2 | s585/post2/vita (final, review round 2) | 6f9c776f1e234a5dd04609f67302000f | release | wt-s585 + slice2.patch (git diff md5 4a3813a5b19e49d893826896d55c0ccc) | s585/r2/diff/REPORT.md:3-4 (= md5_at_preservation) |
| JIT1 | s585/tgt-jit/release/vita as built 09:00 (overwritten 11:06) | 34f9d5da… (prefix only) | release, `-p cli --features jit` | unrecorded (sources "UNVERIFIED") | s585/r1/sound/REPORT.md:142 |
| JIT2 | s585/tgt-jit/release/vita | 3f20e9a0eaccc99539e71cf1f19d35d4 | release, `--features jit` | wt-s585 + slice2.patch | s585/r2/diff/REPORT.md:4 prefix; full = md5_at_preservation |
| NODEF1 | s585/tgt-nodef/release/vita as built 09:01 (overwritten 11:07) | 7eaa30f8… (prefix only) | release, `--no-default-features` | unrecorded | s585/r1/sound/REPORT.md:142 |
| NODEF2 | s585/tgt-nodef/release/vita | f97f67cf3a0daeea5a19c81925570a42 | release, `--no-default-features` | wt-s585 + slice2.patch | s585/r2/sound/REPORT.md:4 prefix; full = md5_at_preservation |
| PRE-NODEF | s585/tgt-pre-nodef/release/vita (mtime 09:06:49) | 265594a76745b101a6e1eaf8fc15fe9e | release, `--no-default-features` | git archive 75f255d4 (s585/PLAN.md:122-123) | md5_at_preservation |
| unrecorded | binary of r1/diff/w/q4/o.txt | unrecorded | unrecorded | unrecorded | unrecorded |
Tag of a `POST` section inside an .out = the binary its harness names: g/out=POSTC (g/r.py:8), g/out2=POST
(g/r2.py:12), g/out3=POST2 (g/r3.py:102), r1/*/out=POST (r1/sound/r.py:8, r1/diff/h.py:12), r2/*/out=POST2
(r2/sound/r.py:8, r2/diff/h2.py:12); `POSTJIT` = JIT1 in r1/diff/out, JIT2 in r2/diff/out. Lane files: r1/sound/jit
.def=POST .jit=JIT1 .nod=NODEF1 .predef=PRE .prenod=PRE-NODEF; r2/sound/lanes .def=POST2 .jit=JIT2 .nod=NODEF2
(from the REPORT headers and mtimes; the command that wrote them is unrecorded). VCD `st.vcd` = staged vrun, tag
`<tag>-staged`; VCD `l_*.vcd` = writer unrecorded (tag unrecorded).

## Oracles
- verilator: "verilator 5.052 --binary --timing --assert +verilator+error+limit+1000" (s585/GROUNDING.md:4);
  build banner "Verilator 5.052 2026-09-05 rev vUNKNOWN-built20260905" (s585/docs_cells/vl_k1w_comb_display.log:8).
- iverilog: "iverilog 13 -g2012" (s585/GROUNDING.md:4); "iverilog 13.0 -g2012" (s585/COMMIT_SLICE.txt:7).
- sv2v: "sv2v 0.0.13 = S/s580/sv2v/sv2v-macOS/sv2v -> iverilog" (s585/GROUNDING.md:5).
- xcelium: not used.

## Verdict sources
verdict_class is quoted, in this order of precedence, from: lens finding headers (s585/r1/diff/REPORT.md:48,61,68,
79,83; s585/r1/sound/REPORT.md:18,37; s585/r2/diff/REPORT.md:30; s585/r1/diff/REPORT.md:15,56 for q4/ivp); the
final 396-cell census s585/g/cmp3_final.txt lines 5-8 (category label before the colon); s585/DOCS_ROWS.md rows
10 (🆕 AB), 14 (🆕 AA), 18 (Oracle splits), 22 (unique-if-chain), 26 (unique-pkg-closure), 36 (PROBE_CATALOG)
(cell named in the row text, `_H` folded to the cell); planning-era pin classes s585/p/PINS.txt (`########`
header of the `#### cell` block; written for option c2, before the round-1 cut). Several labels are joined with
` | `, sources in the same order. A cell in no list = `unrecorded` (cmp3_final.txt is a census: an unlisted cell
had none of its four categories, but no line says so per cell).

## Layout
One harness pattern for every set: `<dir>/out/<name>.out` is a COMBINED text file (role `combined`) written by
the set's runner: `=== verilator …` / `=== iverilog …` / `=== sv2v->iverilog …` sections are the raw oracle
stdout (+compile rc), `=== vita <TAG> …` sections the raw vita stdout per backend (merged when native=interp=vm),
`--- …` lines the runner's own comparisons. The run dir `<dir>/w/<name>/` holds the copied source(s), `s2v.v`
(sv2v translation, role oracle_sv2v_translation), VCDs (role vcd) and excluded build trees (obj/, st_*/ .vu/.velab,
*.vvp). g/out3 copies the oracle sections verbatim from g/out, r1/sound/out, r1/diff/out (or re-runs verilator,
marked `[run by r3]`). A cell = the md5 of the main source of an .out that has at least one oracle section;
its sv row points to the earliest authoring copy (g/c, g/c3..c7, p, r1/sound/cells, r1/diff/cells, r2/*/cells,
lanes, docs_cells; run-dir copies only when nothing else exists: g/w/f1c_qual). Tag column of a combined row =
comma list of its oracle sections then its vita tags.
| set | dir | cells from | example |
|---|---|---|---|
| g/c, g/c3, g/c4, g/c5, g/c6, g/c7 | grounding cells | g/out, g/out2, g/out3 (g__) | g/c/a00_repro.sv; g/out/a00_repro.out, g/out2/a00_repro.out, g/out3/g__a00_repro.out |
| p | planner cells (generated by p/gen.py, gen2.py, gen3.py; `_H` = PRE-H spelling) | g/out* | p/t1s_m_fg.sv |
| r1/sound/cells (+q1d_inc, q1e_multi, q3vu) | soundness lens r1; `_case` = iverilog `unique case` twin (its own cell) | r1/sound/out, g/out3 (s__) | r1/sound/cells/q6a_fn_vfn_noformal.sv; g/out3/s__q6a_fn_vfn_noformal.out |
| r1/diff/cells (+eq) | differential lens r1; sidecars .files/.srcs/.vargs/.vlargs (role aux) | r1/diff/out, g/out3 (d__), r2/diff/out | r1/diff/cells/q23_prio_u0.sv |
| r2/sound/cells, r2/diff/cells | round-2 lens cells | r2/*/out | r2/diff/cells/r2a_bare_forms.sv |
| lanes/j, lanes/n | lane copies (all aliases) | r2/sound/lanes (J1) | lanes/n/vl_lines.sv |
| r1/diff/w/q4 | flags experiment; verilator raw in vr.txt/vr2.txt (role oracle_verilator), vita o.txt | - | r1/diff/w/q4/q4.sv |
| r1/diff/w/ivp, docs_cells | oracle raw quoted in a doc only (doc-quote) | - | r1/diff/w/ivp/p.sv |
Suffix → role: `.out` combined (or vita when it has no oracle section: r1/sound/out q3p/q6a/q6t, r2/diff/out 40);
`s2v.v` oracle_sv2v_translation; `vr*.txt` oracle_verilator; `.def/.jit/.nod/.predef/.prenod`, `{pre,post}_{be}.{log,clean}`,
`eq/*/{pre,post}.txt`, `nodef/*.txt` vita; `*.vcd` vcd; `_H.sv` / extra sources / sidecars aux.
VCD flag: 104 VCD files (63,544 B) are listed with role vcd; the repo .gitignore has `*.vcd` (parent decides).

## Counts
- Cells: 432 rows = 402 kept + 30 aliases. Kept per set: g/c 56, g/c3 38, g/c4 4, g/c5 56, g/c6 35, g/c7 24,
  g/w/f1c_qual 1, p 65, r1/sound/cells 65 (+q1d_inc 1, q1e_multi 1, q3vu 2), r1/diff/cells 33 (+eq 4),
  r2/sound/cells 9, r2/diff/cells 5, r1/diff/w/q4 1, r1/diff/w/ivp 1, docs_cells 1. Aliases: g/c5 6, r1/sound/cells 2,
  r1/diff/cells 3, lanes/j 2, lanes/n 16, r1/diff/w/q4 1.
- oracles_present (kept): iverilog,verilator 190; verilator 183; iverilog,verilator,sv2v 21; iverilog 4;
  verilator,sv2v 2; doc-quote 2 (ivp/p, q18p); unrecorded 0. Aliases: 10 with outputs of their own
  (iverilog,verilator 7; verilator 3), 20 with `-`.
- verdict_class / verdict_source: recovered 163 kept + 17 alias; unrecorded 239 kept + 13 alias.
- vita_tags: recovered 399 kept; unrecorded 3 (q4: binary unrecorded; ivp/p and q18p: no vita output kept).
- binary md5: full for PRE, POSTC, POST, POST2, JIT2, NODEF2, PRE-NODEF; prefix only for JIT1, NODEF1; 1 tag unrecorded.
- base commit: recorded for PRE, PRE-H, POSTC, POST, POST2, JIT2, NODEF2, PRE-NODEF; unrecorded for JIT1, NODEF1.

## Excluded (EXCLUDED.tsv, bytes)
build-dir 978,096,577 (tgt-jit, tgt-nodef, tgt-pre-nodef, verilator obj trees); built-binary 29,352,064 (4 vita);
repo-copy 22,834,398 (pre-src); vita-obs-json 20,682,201 (14,152 run.json/jsonl); third-party 11,869,771 (corpus
.vu + picorv32.v); cargo-gate-corpus-mutation-log 9,552,910; no-oracle-output 9,375,324 (235 files); vita-staged
4,993,643; harness-copy 4,305,136 (7,810 md5-identical copies); rust-patch-or-test-draft 595,613; superseded-log
403,031 (g/q*.log); vvp 296,201; run-metadata 228,316; doc-draft 209,744; vita-only-sweep-log 68,710; script
45,315; verilator-build-log 26,598; progress-log 25,593; duplicate 12,142 (g/cmp3_post2.txt).

## Not recoverable
- docs_cells k1w_comb_display.sv, k2bw_comb_assert.sv: oracle runs summarized only ("docs_cells k1w, k2bw: vita =
  iverilog, exit 0", s585/DOCS_ROWS.md:10); no raw iverilog/verilator run output exists (vl_*.log = build log) →
  EXCLUDED no-oracle-output.
- r1/diff/w/ivp/p.sv iverilog output and docs_cells/q18p iverilog output: quoted in docs only (kept as doc-quote cells).
- PRE-H runs of q6a/q6t (r1/sound/REPORT.md:29) not saved; their `_H.sv` kept as aux.
- r1-time verilator raw for q1d_include / q1e_multi (r1/sound/cells/*/vlc.log is the build log only) and for
  q3p/q6a/q6t (r1/sound/out has no oracle section): only the g/out3 `[run by r3]` verilator run exists.
- JIT1 / NODEF1 binaries overwritten (md5 prefixes only).
- Vita-only experiments (deep, elim, jit/hj, perf, lib, lib2, msg, q7b, eq/obs sweeps) have no oracle: excluded by rule.

## Size
- Manifest (excluding role=alias): 2,152 files, 3,258,668 B. Alias rows: 30 (not copied).
- By role: combined 1090 / 2,380,855; vita 356 / 240,179; sv 402 / 230,379; verdict_table 14 / 206,068;
  aux 159 / 70,803; vcd 104 / 63,544; runner 7 / 43,822; generator 3 / 12,724; oracle_sv2v_translation 15 / 9,682;
  oracle_verilator 2 / 612.
- By set (files / bytes, outputs counted with their cell): g/c 238/341,609; g/c3 199/222,097; g/c4 19/29,729;
  g/c5 262/476,574; g/c6 140/165,205; g/c7 110/227,296; g/w/f1c_qual 4/9,400; p 336/557,328;
  r1/sound/cells 444/411,701 (+q1d_inc 15/4,640, q1e_multi 15/3,555, q3vu 4/11,021); r1/diff/cells 159/398,004
  (+eq 120/92,382); r2/sound/cells 38/19,182; r2/diff/cells 10/20,872; r1/diff/w/q4 4/1,184; r1/diff/w/ivp 1/188;
  docs_cells 1/458; lanes/j 3/1,049; set-level (runners, generators, verdict tables) 30/265,194.
- 10 largest: GROUNDING.md 53,175; p/PINS.txt 52,204; r1/diff/out/q10_cfork_ca.out 42,441;
  g/out3/d__q10_cfork_ca.out 42,327; PLAN.md 22,936; r2/diff/out/s3b_foreach.out 18,125; r1/diff/REPORT.md 17,315;
  r1/sound/cells/q5_long_chain250.sv 16,872; r1/sound/REPORT.md 15,123; g/out3/d__s3b_foreach.out 14,167.
- No file > 1 MB; no set > 5 MB.
- Third-party: none manifested (screen of every manifested .sv/.v/.svh against thirdparty_modnames.txt hits only
  the generic names top/dut/tb; no license text). Corpus .vu snapshots and the picorv32.v copy are EXCLUDED third-party.
