# s583 oracle-cell census (phase 1: manifest only)

## Slice
- Slice: §4.5.583. ROADMAP rows: §3.b unique-glitch-t0 and §3.b unique-if-chain ("§4.5.583 — §3.b unique-glitch-t0 and unique-if-chain, the external report's first queue row." s583/COMMIT_MSG.txt:3).
- Outcome as recorded: "Reverted in review round 1; no code ships." (s583/COMMIT_MSG.txt:4); commit subject "docs: §4.5.583 reverted — `unique if` chains wait for §2 🆕 Z's time-0 order; both oracles refute the end-of-step deferral" (s583/COMMIT_MSG.txt:1; repo commit 63b9dbb9).

## Binaries
| tag | path under $S | md5 | profile | base commit / branch | md5 source |
|---|---|---|---|---|---|
| PRE | s583/pre/vita | 2492e1c5c52c7081a6c0997fe194af40 | release ("cargo build -p cli --locked --release") | HEAD bf7f3e0d | s583/GROUNDING.md:8 (base: GROUNDING.md:3) |
| POST | s583/post/vita | 41566ca6a6c8eaf9a232bf5741a7058c | release | bf7f3e0d + the reverted attempt (working tree of the main checkout, j/postjit_build.log paths); branch unrecorded | s583/r1/diff/REPORT.md:4 |
| PRE-jit | s583/pre-jit/vita | bdac9fa485bd34ff2140bcd4d4b44eea | release, --features jit (j/prejit_build.log "Finished `release`") | pre-src snapshot (bf7f3e0d) | s583/r1/diff/REPORT.md:5 |
| POST-jit | s583/post-jit/vita | 9d508f2d04a2938f7fd09dfa218c28f3 | release, --features jit | attempt tree | s583/r1/diff/REPORT.md:5 |
- Tag forms: `PRE/obs`, `POST/obs` = run.json + results.jsonl observability files of a run (m/sw/*/obs_PRE|obs_POST, r1/diff/wr/*/obs_pre|obs_post); `PRE/staged-*` = plan/st_* vcmp/velab/vrun logs; `PRE/-Werror=W4031`, `POST/-Werror=W4031` = rm583/wpost/werr_*.txt; `PRE(H-spelling)` = the `--preh` PRE run on the `_H.sv` aux inside a combined .out.
- plan/obs_* and plan/st_* carry tag PRE: written 15:57 (plan/obs_b02/run.json mtime), before s583/post/vita existed (16:12), and PLAN.md:20 says "H-cell trick: on PRE". Flagged as inferred from mtime + plan text.
- rm583/*.out vita sections: run.py VITA = s583/pre/vita (s583/run.py:8) -> PRE.

## Oracles
- Captured: "Measured (iverilog 13 -g2012; verilator 5.052 --binary --timing --assert, run with +verilator+error+limit+1000; vita PRE, native = interp = vm)" (s583/COMMIT_MSG.txt:6-7); "sv2v 0.0.13 -> iverilog" (s583/GROUNDING.md:5). sv2v binary: s580/sv2v/sv2v-macOS/sv2v (s583/run.py:9). sv2v ran on a00, a01, b00 only ("run on a00, a01, b00 only", GROUNDING.md:23).

## Verdict sources
- s583/GROUNDING.md cell tables (A and B), last column "verdict"; a cell with several rows (t1, t2, ...) gets the distinct verdicts joined with " / " (notes say how many rows).
- s583/r1/diff/REPORT.md cell table (last column "class / new?") and findings prose; s583/r1/sound/REPORT.md findings prose (first line naming only this cell; first verdict word of: real gap, NON-BLOCKING, BLOCKING, agree, vita-wrong, pre-existing, NEW, Control, Boundary, Residue, Lane check; else the `###` heading's word).
- m/sweep.txt: PRE-vs-POST sweep table over m/sw (manifested as verdict_table; not used for verdict_class).
- Alias cells take the kept cell's verdict (notes say so). rm583, plan, m/*.sv and m/sw plan_* copies: no per-cell verdict in any slice doc -> unrecorded.

## Layout
- c/<name>.sv -> c/<name>.out (role `combined`: s583/run.py transcript holding raw sections `=== iverilog` (iverilog -g2012 + vvp), `=== verilator` (--binary --timing --assert), `=== sv2v->iv`, `=== vita (native=interp=vm)` = PRE; tag lists the sections present, e.g. `iverilog+verilator+PRE`) and c/w_<name>/sv2v.v (oracle_sv2v_translation; a.vvp/b.vvp/obj_vl excluded). Example: c/a00_repro.sv -> c/a00_repro.out, c/w_a00_repro/sv2v.v. rm583/ and rm583/g/ use the same layout (rm583/g = byte-identical copies of c/ cells -> aliases).
- r1/diff/cells/<name>.sv -> r1/diff/cells/<name>.out (role `combined` from r1/diff/r.py: `=== verilator compile … run …`, `=== vita PRE …`, `=== vita POST …`, `=== vita PRE-H …`); `<name>_H.sv` = aux of `<name>.sv` (the --preh PRE spelling inside the same .out). r1/diff/w/<name>/<name>.sv and r1/diff/wr/<name>/<name>.sv are copies (aliases); wr/<name>/obs_pre|obs_post hold vita PRE/POST run.json+results.jsonl. r1/diff/wiv/c22c_case_only.sv = alias.
- r1/sound/cells/<name>.sv -> r1/sound/cells/vlc_<name>.log (oracle_verilator compile log) + aggregate files vl.out (verilator, sections per cell), pre.out, post.out (vita) manifested once as set-level rows (cell_id `-`); cells whose section exists in vl.out count verilator as present.
- m/<name>.sv, plan/<name>.sv, j/J1.sv -> m/vlc_<name>.log, plan/vl_<name>.log, j/vl_J1.log (oracle_verilator: verilator compile output written by m/vl.sh / plan runs); m/sw/<c|j|m|plan>_<name>/<name>.sv = copies (aliases) with obs_PRE / obs_POST vita observability outputs; sweep verdicts in m/sweep.txt.
- Doc-quote cells (no oracle section in their .out; verilator / iverilog raw text quoted in r1/diff/REPORT.md): r1/diff/cells/c17_pkg_task_import.sv (table row line 66, verilator column `pk.pta :5 t1; prio silent`; aux c17_pkg_task_import_H.sv) and r1/diff/cells/c22c_case_only.sv (line 35, "iverilog (case analog c22c, raw): …"); their copies in r1/diff/w, r1/diff/wr, r1/diff/wiv are aliases.
- Combined-file role: `combined` is not in the SPEC role list; one file carries iverilog/verilator/sv2v raw text and vita text together, so it cannot be split without rewriting. Parent decides the role name.

## Counts
- Cells: 213 (aliases 104; oracles_present=doc-quote 5)
- Per set: c 46; j 1; m 3; m/st 1; m/sw 56; main 1; plan 7; r1/diff/cells 27; r1/diff/w 25; r1/diff/wiv 1; r1/diff/wr 11; r1/sound/cells 6; rm583 20; rm583/g 8
- Unrecorded per field: oracles_present 0/213; vita_tags 47/213; verdict_class 55/213; verdict_source 55/213
- binary md5: all four recorded in slice docs. base commit: PRE recorded (bf7f3e0d); POST branch unrecorded.

## Not recoverable / not cells
- plan H/S/Hm spellings (b02H, b03H, ca_H, fold_H, l07f_H, l07f_S, lt_H, l08_H, l08_Hm) and their obs_/st_ outputs and m/sw/plan_* copies: vita-only PRE spellings of the plain P cells (PLAN.md:20 "Each lane compares PRE-on-H with verilator-on-plain") -> excluded as no-oracle-output; parent may decide otherwise.
- j/J1_H.sv, rm583/e06_dut_chain_H.sv, r1/diff/wt/c22t.sv (trace), r1/sound/cells c2b/c2c/c3h/c7, main/single_else.sv, main/d02_plain.sv: no oracle output and no oracle quote naming them.
- Verilator run output for m/ and plan/ cells was printed by m/vl.sh to the console only; only the compile logs (vlc_*/vl_*.log) were saved.
- ibex corpus run (ibex_cmd.txt, ibex.out, ibex.err): excluded as third-party corpus.

## Size
- Manifest total (excluding role=alias): 506 files, 529188 bytes (0.50 MiB); alias rows: 115
- By role: vita 263 files / 219280 B; combined 91 files / 112300 B; oracle_verilator 24 files / 100448 B; sv 109 files / 52753 B; verdict_table 1 files / 21803 B; runner 7 files / 16564 B; aux 8 files / 4868 B; oracle_sv2v_translation 3 files / 1172 B
- By set:
  - m/sw: 213 files / 171362 B
  - c: 97 files / 83061 B
  - r1/diff/cells: 59 files / 72794 B
  - plan: 24 files / 62534 B
  - r1/diff/wr: 40 files / 33340 B
  - r1/sound/cells: 12 files / 25272 B
  - set-level (verdict_table): 1 files / 21803 B
  - rm583: 42 files / 17102 B
  - set-level (runner): 7 files / 16564 B
  - m: 6 files / 10574 B
  - set-level (vita): 2 files / 8825 B
  - j: 2 files / 4627 B
  - set-level (oracle_verilator): 1 files / 1330 B
- 10 largest manifested files:
  - m/sweep.txt (verdict_table) 21803 B
  - m/vlc_vl_lines.log (oracle_verilator) 6675 B
  - plan/vl_lines.log (oracle_verilator) 6675 B
  - r1/sound/cells/post.out (vita) 4847 B
  - m/vlc_lt_P.log (oracle_verilator) 4543 B
  - plan/vl_lt_P.log (oracle_verilator) 4543 B
  - r1/sound/cells/vlc_c3p_class_fn.log (oracle_verilator) 4542 B
  - m/sweep.py (runner) 4485 B
  - m/vlc_b01_nested_qual.log (oracle_verilator) 4433 B
  - m/vlc_m_if_unique0.log (oracle_verilator) 4433 B
- Files > 1 MB: 0
- Sets > 5 MB: 0
- VCD rows (role vcd; repo .gitignore has *.vcd, parent decides): 0 files / 0 B

## Third-party screen
- Manifested .sv/.v module names vs $S/preserve/thirdparty_modnames.txt: no match other than generic names (top); no upstream text markers. third-party-suspect: none. Excluded corpus: s583/ibex_cmd.txt, s583/ibex.out, s583/ibex.err.
