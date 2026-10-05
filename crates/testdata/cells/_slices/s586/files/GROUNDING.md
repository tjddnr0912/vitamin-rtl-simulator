# GROUNDING §4.5.586 — §3.b unique-overlap-note (HEAD fc1fc07e)

Status: COMPLETE (G1-G7 + open decisions)

PRE: /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s586/pre/vita md5=8d33349579ca773389265f7a888fd17e size=7338016 (release, cargo build -p cli --release --locked @fc1fc07e). Staged: /private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s586/pre/sep/{vita,vcmp,velab,vrun} (--features separate-bins) md5 vcmp=a6c25fc8 velab=fed435ff vrun=f607ba50 vita=f758e6dc

## G1 repro 3 tools

Cells: S/g/cNN_*/ (t.sv, vita.{out,err,rc}, iv.{cout,cerr,out,rc}, vl.{cout,cerr,out,rc}).
Runner: S/run3.sh (vita PRE; iverilog -g2012 + vvp -n; verilator --binary --timing --assert -Wno-fatal --top-module t, run +verilator+error+limit+1000).
Every vita cell also prints `warning[VITA-W1017] W-PP-TIMESCALE-DEFAULT ...` and summary `errors=0 warnings=1 notes=0` (omitted below). "nothing" = no line about unique.

| cell | shape | vita PRE | iverilog 13.0 | verilator 5.052 |
|---|---|---|---|---|
| c01_ucasez | unique casez ?1/1?, r=11 | y=1 rc0, nothing | stderr at compile `t.sv:6: vvp.tgt sorry: Case unique/unique0 qualities are ignored.`; y=1 | `[1] %Error: t.sv:6: Assertion failed in t: unique case, but multiple matches found for '2'h3'`; lint `%Warning-CASEOVERLAP: t.sv:8:7: Case conditions overlap (example pattern 0x3)`; y=1 |
| c02_u0casez | unique0 casez | y=1, nothing | sorry t.sv:6 | `[1] %Error: t.sv:6: Assertion failed in t: unique0 case, but multiple matches found for '2'h3'`; CASEOVERLAP |
| c03_ucase | unique case 01/01 | y=1, nothing | sorry t.sv:6 | `... unique case, but multiple matches found for '2'h1'`; CASEOVERLAP |
| c04_u0case | unique0 case 01/01 | y=1, nothing | sorry t.sv:6 | `... unique0 case, but multiple matches found for '2'h1'` |
| c05_uif | unique if(a)...else if(b), a=b=1 | y=1, nothing | crc4 `t.sv:6: syntax error` / `t.sv:6: Syntax in assignment statement l-value.` | `[1] %Error: t.sv:6: Assertion failed in t: 'unique if' statement violated`; y=0 (!) |
| c06_u0if | unique0 if, both true | y=1, nothing | crc4 syntax error | `[1] %Error: t.sv:6: Assertion failed in t: 'unique if' statement violated`; y=0 |
| c07_pcase | priority casez overlap | y=1, nothing | y=1, NO sorry | y=1, nothing |
| c08_pif | priority if both true | y=1, nothing | crc4 syntax error | y=1, nothing |
| c09_ucase_noovl | unique case full, no overlap | y=1, nothing | sorry t.sv:6 | nothing |
| c10_func | unique casez in module function, called | y=1, nothing | sorry t.sv:4 | `... Assertion failed in t.f: unique case, but multiple matches ...` |
| c11_task | in task, called | y=1, nothing | sorry t.sv:4 | `... in t.tk: unique case ...` |
| c12_class | in class method, called | y=1, nothing | sorry t.sv:4 | `... in $unit.C.m: unique case ...` |
| c13_uninst | always_comb unique casez in module `unused`, never instantiated, no --top | y=5; W3057 auto-top elaborates `unused` too; `t.sv:3:12: warning[VITA-W4031] W-RUN-UNIQUE-VIOLATION: value is unhandled for priority or unique case statement [in unused] [at time 0]` | sorry t.sv:3 (iverilog roots it) | (--top-module t) nothing |
| c13b_uninst_top | same, vita `--top t`, iverilog `-s t` | y=5, nothing | NO sorry | nothing |
| c14_gen_untaken | unique casez in `if (P==1)` generate, P=0 | y=5, nothing | NO sorry | nothing |
| c15_pkgfn | unique casez in package function, `pk::f(r)` | rc1 `t.sv:16:5: error[VITA-E3009] E-ELAB-UNSUPPORTED: package-scoped call `pk::f(...)` needs a body ...` (= §3.b unique-pkg-closure) | sorry t.sv:4 | `... in pk.f: unique case ...` |
| c16_two_sites | unique casez + unique0 casez in one block | y=1 z=1, nothing | sorry t.sv:6 AND sorry t.sv:10 | both (t.sv:6 unique, t.sv:10 unique0) |
| c17_uncalled_fn | unique casez in a function never called | y=5, nothing | sorry t.sv:4 (printed) | nothing |
| c18_ucase_default | unique casez overlap + default | y=1, nothing | sorry t.sv:6 | `... unique case, but multiple matches ...` |
| c19_always_comb | always_comb unique casez + default, r=11 | y=1, nothing | sorry t.sv:4 | `[0]`, `[1]`x2, `[2]`x2 `... unique case, but multiple matches found for '2'h3'` (per evaluation) |
| c20_inside | unique case inside [0:7]/[4:9], r=5 | y=1, nothing | crc6 syntax error (case-inside ranges) | `... unique case, but multiple matches found for '32'h00000005'` |
| c21_noovl_if | unique if, a=1 b=0 | y=1, nothing | crc4 syntax error | nothing |
| c22_priority_only_noovl | priority case + default | y=1 | NO sorry | nothing |
| c23_two_unique_noovl | two unique case, no overlap | y=1 | sorry t.sv:6 + sorry t.sv:11 | nothing |
| c24_two_inst | module with unique casez instantiated twice | y1=1 y2=1, nothing | sorry `t.sv:3` TWICE (identical lines) | per instance `in t.u1`, `in t.u2` |
| c25_multiline | `unique` / `casez` / `(r)` on lines 6/7/8 | nothing | sorry `t.sv:6` (qualifier keyword line) | `t.sv:7` (`casez` keyword line) |

iverilog `sorry` (measured): stage = compile (vvp.tgt code generator); stream = stderr of `iverilog`, before vvp runs; exit code unchanged (crc=0); format `<file>:<line>: vvp.tgt sorry: Case unique/unique0 qualities are ignored.`, line = qualifier keyword line, no column; once PER ELABORATED case-statement instance (c16/c23: 2 sites -> 2 lines; c24: 1 site x 2 instances -> 2 identical lines); printed for unique AND unique0 (c02, c04); with or without overlap (c09, c23); with a default (c18); for an uncalled function (c17), a class method (c12), a package function (c15); for an uninstantiated module only when iverilog auto-roots it (c13 yes, c13b `-s t` no); NOT in an untaken generate branch (c14); never for `priority` (c07, c22). `unique if` / `unique0 if` / `priority if` rejected at parse (syntax error: c05, c06, c08, c21), so iverilog has no `if` precedent.

verilator (measured): runtime `%Error ... Assertion failed in <scope>: unique case, but multiple matches found for '<val>'` / `unique0 case, ...` / `'unique if' statement violated` (same text for unique0 if, c06); per evaluation (c19), per instance (c24); location = `case` keyword line; plus compile-time lint CASEOVERLAP for constant items; silent on priority overlap (c07, c08). Quirk: c05/c06 y=0 (neither branch) where IEEE says the first true condition's statement runs (vita y=1).

IEEE 1800-2017 (recalled wording; no local copy of the standard; the repo cites the same clauses at docs/manual/006_limitations.md:174, docs/preview/15-error-code-reference.md:1288):
- §12.4.2: "A unique-if and unique0-if assert that there is no overlap in a series of if-else-if conditions ... A unique-if or unique0-if is violated if more than one condition is found true. The implementation shall issue a violation report and execute the statement associated with the true condition that appears first in the if statement". "If the keywords unique or priority are used, a violation report shall be issued if no condition matches unless there is an explicit else." "A priority-if indicates that a series of if-else-if conditions shall be evaluated in the order listed" (overlap is legal; no overlap report).
- §12.5.3: "Unique-case and unique0-case assert that there are no overlapping case_items ... Unique-case and unique0-case are violated if more than one case_item is found to match the case_expression. The implementation shall issue a violation report and execute the statement associated with the matching case_item that appears first". "If the case is qualified as priority or unique, the simulator shall issue a violation report if no case_item matches." "A priority-case shall act on the first match only."
- => overlap check = unique + unique0 (case and if). No-match check = unique + priority. priority has NO overlap violation (controls c07/c08: both oracles silent).

## G2 diagnostic infrastructure census

Render path (one sink for all stages):
- Model: `crates/diag/src/event.rs:27` `Diagnostic { severity, code: MsgCode, message, location: Option<SourceLoc>, context: Vec<Frame>, sim_time }`; `crates/diag/src/severity.rs:3` 5-level `Note|Info|Warning|Error|Fatal`, token `note` at :15.
- Gate: `crates/vita-log/src/lib.rs:114-138` `GatedSink::emit`: Error/Fatal never gated; Warning suppress (`-Wno-`) or promote (`-Werror[=]`); `_` (Info AND Note) suppressible by `-Wno-<CODE>`, never promoted.
- Printer: `crates/cli/src/lib.rs:304-350` `StderrSink::render_diagnostic`: line = `<file>:<line>:<col>: ` (if location) + `{token}[{code_num}] {mnemonic}: {message}{ [in ctx]}{ [at time N]}`, written to STDERR (`err_write`) and teed to `--log`. Counters :305-309: Note and Info both increment `notes`. Epilogue :287-298 `errors={E+F} warnings={W} notes={Info+Note}` on stderr, unsuppressible.
- `-q` (verbosity 0) silences only Progress/RtlOutput on stdout (:357-371); diagnostics always print. Exit code = `had_error_or_fatal` (:257); a Note/Info never changes exit (manual 007 Severity table, line ~141).
- Caps (manual 007 "Diagnostic caps"): parser errors 50 (warnings uncapped), elaborate errors 200 (`note_at` returns early past it: `crates/elaborate/src/driver.rs:385-389`), runtime index reports 8. No cap applies to parse warnings.

Existing Note-severity emitters: exactly ONE — `crates/elaborate/src/driver.rs:384` `note_at(code, span, msg)` (`severity: Severity::Note` at :395), a follow-on that reuses the PARENT error's code (e.g. `note[VITA-E3009]`, pinned `crates/cli/tests/da_chain_and_unwritten.rs:285`, manual 007:74, manual 006:340). No `MsgCode` row has default severity Note; Info rows = `I-ELAB-USER-INFO` VITA-I3006, `I-RUN-USER-INFO` VITA-I4005 only (user `$info`). `crates/diag/src/code.rs:76-147` table has 70 rows.
- Manual 007 Severity table defines Note as "Follow-on detail for the error above it. Carries that error's code. Never affects the exit code." — a standalone note contradicts that sentence (doc edit required if Note severity is used).
- doc15 governance (`docs/preview/15-error-code-reference.md:26-27`): "The severity letter `S` is `E`=Error, `W`=Warning, `I`=Info, `F`=Fatal" — there is NO `N` letter. :40-45 already says `-Wno-` drops "a Warning, Info or Note"; "Info and Note are suppressible but not promotable". Status table :81-99: "A code's severity is chosen by its emitter, not by the code."

Parser-stage non-fatal channel (the template):
- `crates/hdl-parser/src/lib.rs:152-162` `ParseWarn { span, kind: ParseWarnKind }`, one kind `NonStandardSelectBase`; pushed by `warn_select_base` :1144-1155 (dedup by span).
- `crates/hdl-parser/src/api.rs:23-39` `parse_with_warnings` returns `(unit, errors, warnings)`; `parse()` :14 drops warnings (44 callers).
- Only consumer: `crates/cli/src/frontend.rs:338-367` — maps each `ParseWarn` to `Severity::Warning`, `MsgCode::ParseSelectBase` (VITA-W2004), location via `loc_from_span(&pp.map, …)`, emitted BEFORE the parse-error gate.

Adding a code (CONTRIBUTING.md "Diagnostics" :183-215, three parts):
1. row in `msgcodes!` `crates/diag/src/code.rs` (mnemonic, `VITA-<S>####`, default severity, title);
2. body entry in `docs/preview/15-error-code-reference.md` with header `### VITA-X#### · `MNEMONIC` (Severity)`; that file is `include_str!`'d into the binary (`crates/cli/src/lib.rs:398` `ERROR_CATALOG`, `vita explain`);
3. row in `docs/manual/007_error-codes.md` band table (2xxx table at :368-382, 4xxx at :527-562).
Gate: `crates/diag/tests/bijection.rs:62-74` pins `enum_codes.len() == 70` (must become 71), mnemonic set 1:1 with doc15 body, number + severity per entry (:79-108; severity compared as `token()` vs doc `(Severity)` lowercased, so `(Note)` would parse), uniqueness :111-123.
Also user-facing => CHANGELOG.md `## [Unreleased]` section (house style: `### Fixed — …` / `### Added — …` heading + bullets; top entries at CHANGELOG.md:10-40) and docs/manual (CONTRIBUTING Workflow :291-293).
Free numbers: 2xxx body uses E2001 E2002 W2003 W2004; Appendix A reserves 2004-2020 (E2004…W2020), so next free 2xxx = 2021. 4xxx body ends W4031, Appendix A reserves 4008-4015 => next free 4032. 3xxx body max W3060.
Other code-name enumerations to grep when adding: `docs/preview/13-diagnostics-and-logging.md` (iterates `MsgCode::ALL` prose; table row for W4031 at :838), manual 007 "Codes with no emitter" / "emitted at a severity other than default" tables (:603-626) if Note severity is emitted from an Info/other-letter code.

## G3 pipeline census

Where the qualifier is visible:
- Lexer: `crates/hdl-lexer/src/lib.rs:689` `"unique" => Unique` (+ `unique0`, `priority`, `priority0` keywords).
- Parser, single statement funnel: `crates/hdl-parser/src/stmt.rs:165-167` `Kw::Unique | Kw::Priority | Kw::Unique0 | Kw::Priority0 => self.parse_unique_priority()`; the only caller (grep). `crates/hdl-parser/src/assertions.rs:467-546` `parse_unique_priority`: `qspan = self.cur_span()` at :468 (qualifier token span), `suppress_no_match` true for `unique0`/`priority0` (:473-478); the qualifier kind is otherwise DISCARDED after `self.bump()` (:479).
- Other `Kw::Unique` uses that are NOT qualifiers: `crates/hdl-parser/src/lib.rs:1187-1194` `member_ident` (array locator `.unique()`, defensive `.unique0` / `.priority0` member names). No constraint-block `unique {}` parse site found.
- What survives into AST / IR (measured, S/g/c31_trace/, same file name `t.sv` per dir, staged PRE bins): `unique0 casez` vs plain `casez` -> .velab 502/502 bytes, 32 differing bytes, all at offsets 42-73 (the header's upstream .vu digest, which hashes source text); `unique casez`+default vs plain `casez`+default -> 520/520, 32 differing bytes at 42-73; `unique casez` (no default) vs plain -> 601/502 (the synthesized default arm). So `unique0`, `unique` with an explicit `default` / final `else`, and `priority` with one leave NO trace after parsing. A note decided after parsing (elaborate or engine) needs a new AST carrier: precedent `TopItem::InsideNameUse(Span)` (`crates/hdl-ast/src/lib.rs:165-173`, pushed at `crates/hdl-parser/src/api.rs:34-38`, read at `crates/elaborate/src/driver.rs:647-651`, staged/pipeline merge arms `crates/cli/src/staged.rs:197`, `crates/cli/src/pipeline.rs:598`, `crates/elaborate/src/md_return.rs:37`), which re-pinned `crates/hdl-ast/tests/schema_hash.rs` and made all `.vu` stale with format_version unchanged (commit 69572b42 message / schema_hash.rs:15-23). A parse-time note needs no AST change.

Where a once-per-run note can be emitted (parse-time option):
- Non-fatal channel `ParseWarn` (`crates/hdl-parser/src/lib.rs:152-162`) returned by `parse_with_warnings` (`api.rs:23-39`); the ONE product consumer is `crates/cli/src/frontend.rs:338-367` inside `frontend_pp_to_unit_mapped` (:306), emitted before the parse-error gate.
- Every product parse goes through `frontend_sources_mapped` (`frontend.rs:284`) -> `frontend_pp_to_unit_mapped`: one-shot `vita <srcs>` (`frontend.rs:590`, `run_vita_str_gated`) and `vcmp` / `vita vcmp` (`crates/cli/src/pipeline.rs:519`). `velab` / `vrun` decode `.vu` / `.velab` and never parse. Test-only wrappers `frontend_text_to_unit*` (`frontend.rs:206-279`; used by `crates/cli/tests/case_inside.rs`, `staged_flow.rs`) route to the same function. `hdl_parser::parse` (`api.rs:14`, 44 callers, all sim-engine tests etc.) drops warnings.
- One `Parser` per parse (`api.rs:27` is the only `Parser::new`). No statement-level backtracking: the only `self.pos = save` restores are `type_params.rs:473/487/499/905/912/921`, `expr_primary.rs:216`, `functask.rs:417` (type / `$bits(<type>)` / `const ref` speculation), none reaching `parse_stmt`. So each qualifier token is visited once, in token order.

Caching / replay (measured, S/g/c30_staged/, design with a W2004 select-base warning + unique casez):
- one-shot run twice: identical stderr (`cmp os1.err os2.err` equal), both print W2004 — no compile cache in the one-shot path (no `cache` in `crates/cli/src/*.rs` apart from an obs comment).
- staged: `vcmp` prints `t.sv:5:9: warning[VITA-W2004] …` + W1017 + `errors=0 warnings=2 notes=0`; `velab` prints only `errors=0 warnings=0 notes=0`; `vrun` prints `y=1 c=1`, `simulation ended (Finish) at time 2`, `errors=0 warnings=0 notes=0`. `vita vcmp` same as `vcmp`. => parse-stage diagnostics print at vcmp only and are NOT replayed by velab/vrun. Elaborate diagnostics print at velab (e.g. W3057), runtime at vrun.
- Work-library flow (`vcmp --work`, `velab -L … --top`): each `vcmp` invocation is its own parse; a parse-time once-per-run note prints once per vcmp invocation that contains a site (not measured with two libraries; mechanism = one Parser per invocation).

First-site determinism (parse-time): one-shot preprocesses all argv sources as ONE expanded buffer in argv order (`frontend.rs:284-298`, `preprocess_sources`); `-f`/`-F` filelists expand in file order and refuse wildcards (`E-FLIST-GLOB` VITA-E8004, `crates/diag/src/code.rs`), so no directory-listing order enters; the parser walks tokens linearly. First = first `unique`/`unique0` qualifier token in expanded-buffer order; location resolves through `pp.map` (`loc_from_span`) to file:line:col — no hash-map iteration on the path. Elaborate-time "first reached" would instead depend on `build_module_map` order, instance DFS, package/class lowering order and call-on-demand lowering of subroutines (`driver.rs:640-660`).

Format version: `CURRENT_FORMAT_VERSION = 35` (`crates/vita-artifact/src/header.rs:15`). Parse-time note: no AST / .vu / .velab / SimIr change -> no bump, schema hashes untouched. Elaborate-time carrier: AST shape change -> re-pin `crates/hdl-ast/tests/schema_hash.rs`, `.vu` stale through the schema_hash gate (exit 2), no format_version bump by the InsideNameUse precedent.

Minimal byte-identity argument (parse-time design, designs with no `unique`/`unique0` qualifier): the only new code runs inside `parse_unique_priority`, reached only from `stmt.rs:165` on a `Kw::Unique|Priority|Unique0|Priority0` statement token; with none of `unique`/`unique0` qualifiers it never sets the latch, so `parse_with_warnings` returns the same `Vec<ParseWarn>`; the AST is untouched (no field, no item), so `.vu` body, `.velab`, SimIr, stdout, VCD, exit code and stderr are byte-identical. `priority`/`priority0`-only designs: same, provided the latch keys on `Unique|Unique0` only. Designs WITH a qualifier: one extra stderr line at the parse stage (one-shot / vcmp) and epilogue `notes=0` -> `notes=1` (Note and Info both count, `crates/cli/src/lib.rs:305-309`); stdout, exit code, `.vu`, `.velab`, VCD unchanged. Channels: value / exit class / order / time unchanged; diagnostic stream +1 line +epilogue count.

## G4 blast radius

Source census: `grep -rlE "unique0?[[:space:]]+(case|casez|casex|if)" crates` = 16 files (S/g4_files.txt): cli/src/lib.rs (comment), hdl-parser/src/{assertions,lib}.rs, hdl-parser/tests/{case_inside_shape,unique_if_chain_shape}.rs (call `hdl_parser::parse`, which DROPS `ParseWarn` — api.rs:14), and 11 cli tests: always_comb_t0_after_settle, always_comb_t0_splits, case_inside, gen_block_enum_labels, pkg_frame_span_owner, procedural_adv, round29_report, runtime_diag_location, severity_in_frame_body, unique_if_chain (201 qualifier lines), unique0_priority0. (+ `output_formal_any_position.rs` matched only a multi-line `unique` regex; its hit is prose.)

Measured blast radius (probe, not a proposal): scratch copy `git archive HEAD` -> S/probe_src, CARGO_TARGET_DIR=S/probe_target, patch = ParseWarnKind::UniqueOverlapUnchecked pushed once at the first `unique`/`unique0` qualifier (`parse_unique_priority` before `bump`), frontend emits `Severity::Note`, new code `I-PARSE-UNIQUE-UNCHECKED` VITA-I2021 (doc15 stub, bijection pin 70->71). Message: "`unique` / `unique0` overlaps (more than one matching item or condition) are not checked; only a missing match is reported. This is the first such site; others are not listed".
- command: `cargo nextest run --workspace --locked --no-fail-fast` (probe tree), log S/probe_nextest.log, rc=100.
- `Summary [  41.380s] 9032 tests run: 9020 passed, 12 failed, 15 skipped`; no TIMEOUT/SIGSEGV/ABORT/LEAK-FAIL lines.
- 12 FAIL = 6 `cli::always_comb_t0_after_settle` (a_block_before_its_driver_reports_only_the_real_miss, a_block_behind_a_port_reports_only_the_real_miss, a_block_fed_through_a_port_from_a_declaration_initialised_block_is_silent, a_first_slice_read_sees_the_block_unrun_and_a_hash0_read_sees_it_run, settle_woken_blocks_chained_through_an_assign_or_a_port_run_one_at_a_time, with_no_first_batch_the_settle_woken_blocks_still_run_one_at_a_time) + 6 `cli::always_comb_t0_splits` (processes_the_rule_must_not_move, residue_a_consumer_block_before_its_producer_block_still_reports_at_time_zero, split_a_forward_chain_from_a_quiet_source_reports_nothing_at_time_zero, split_a_settle_woken_block_runs_twice_at_time_zero, split_order_of_the_implicit_pass_against_hash0_and_hierarchical_reads, split_writers_the_rule_leaves_as_they_were).
- Fingerprint (verbatim, after_settle.rs:77): `left: ["5:5 I2021 ", "5:12 W4031 [in top] [at time 2]"]` / `right: ["5:12 W4031 [in top] [at time 2]"]`.
- Mechanism: `crates/cli/tests/always_comb_t0_util/mod.rs:30-46` `diags()` keeps every `[VITA-` stderr line except `W-PP-TIMESCALE-DEFAULT` and the tests assert the exact list. Second latent break in the same util: `staged_matches` (:85-98) asserts staged `vrun` Run (stdout + diags + exit) == one-shot Run; a parse-stage note prints in one-shot but only at `vcmp` in the staged chain (S/g/c30_staged), so after re-pinning, the staged comparison diverges unless `diags()` also filters the note (the house precedent: W1017 is filtered for exactly this reason).
- PRE attribution: same two files at HEAD in the main checkout `cargo nextest run -p cli --locked --test always_comb_t0_after_settle --test always_comb_t0_splits` -> `Summary [   0.795s] 21 tests run: 21 passed, 0 skipped` (S/pre_ac_tests.log). All 12 are probe-caused.
- Every other unique-bearing test (unique_if_chain, round29_report, procedural_adv, runtime_diag_location incl. `staged_vrun_matches_one_shot_byte_for_byte`, unique0_priority0, case_inside, severity_in_frame_body, gen_block_enum_labels, pkg_frame_span_owner) passed with the probe — they assert by `contains` / W4031 counts. Wording-sensitive: a note text containing `W4031`, `unhandled`, or `warning[` was not measured.

Corpus: bench/*/src grep (bash, `*.sv|*.v|*.svh|*.vh`): ibex 180 lines with a qualifier, every other workload 0; `priority` qualifiers 0 everywhere. In the files ibex actually compiles (bench/ibex/files.txt + tb.sv): 123 `unique case` lines, 0 `unique0`, 0 `unique if` (S/g4_ibex_sites.txt); first in argv order = `src/vendor/lowrisc_ip/ip/prim/rtl/prim_secded_pkg.sv:41` (`unique case (sd_type)`, inside a package function); top files: ibex_decoder 23, ibex_compressed_decoder 20, ibex_alu 18, ibex_load_store_unit 12, prim_secded_pkg 10.
- corpus-runner grading (crates/corpus-runner/src/run.rs): digest scanned from STDOUT only (:124-134), exit code compared (:379-388), refused rows match the pinned diag as a substring anywhere in stderr (:145-155, fallback = first line containing `error[` / `error:`). ibex is `Expect::Runs { exit: 0 }` (corpus.rs ~:583). A stderr note line changes none of digest / exit / refusal => no DRIFTED path by construction (measured end-to-end: see G7 ibex cell if run).
- examples/*.sv (4 files) and bench tb files: 0 qualifiers. No example or bench golden captures stderr.

Docs claiming behaviour on overlaps: listed in G6 (manual 006:209-213, manual 003:763, preview 01:136, preview hdl-reference 03-procedural.md:90-92, manual 007 Severity Note row).

## G5 decision inputs

Probe cells (probe release `S/probe_bin/vita_default` md5 9d6736b579a0bb7e6c27a7e58aeaeda1, parse-time first-site design from G4; outputs `S/g/<cell>/probe.{out,err,rc}`):

| cell | probe note | iverilog sorry | verilator report |
|---|---|---|---|
| c01-c04 unique/unique0 case/casez | `t.sv:6:5: note[VITA-I2021] …` ×1 | yes | yes (overlap) |
| c05/c06 unique/unique0 if | t.sv:6:5 ×1 | parse error | yes |
| c07/c08/c22 priority only | none | none | none |
| c42 priority casez then unique0 case | t.sv:9:5 (the unique0, not the priority at :5) | — | — |
| c09/c23 no overlap | t.sv:6:5 ×1 (c23: 2 sites -> 1 note) | yes (per site) | none |
| c10/c11/c12 func/task/class method | t.sv:4:5 | yes | yes |
| c13 uninstantiated, auto-top | t.sv:3:5 | yes | none |
| c13b uninstantiated, `--top t` | t.sv:3:5 | NO (`-s t`) | none |
| c14 untaken generate branch | t.sv:5:7 | NO | none |
| c15 package fn (vita E3009 rc1) | t.sv:4:5, rc1 | yes | yes |
| c17 uncalled function | t.sv:4:5 | yes | none |
| c24 one site ×2 instances | t.sv:3:5 ×1 | ×2 | ×2 (per instance) |
| c25 qualifier/case on lines 6/7 | t.sv:6:5 | t.sv:6 | t.sv:7 |
| c43 a.sv,b.sv vs b.sv,a.sv | `a.sv:2:15` vs `b.sv:6:5` (argv order) | — | — |
| c44 unique then a syntax error | note t.sv:4:5 printed before `t.sv:5:9: error[VITA-E2002] …`, rc1, `errors=1 warnings=0 notes=1` | — | — |
| c45 unique inside `ifdef NOPE` | none; with `-DNOPE`: t.sv:5:5 | — | — |
| c46 `define UCASE(sel) unique case (sel)` | t.sv:6:5 (macro call site) | — | — |
| c47 `timescale` + `-Werror` | rc0, `errors=0 warnings=0 notes=1` (note not promoted) | — | — |
| c40 ibex | `src/vendor/lowrisc_ip/ip/prim/rtl/prim_secded_pkg.sv:41:5: note[VITA-I2021] …` first stderr line; stdout `cmp` identical (DIGEST=13b2ddfcd551ba2f), rc 0/0; epilogue `notes=0` -> `notes=1` | (iverilog cannot parse ibex) | — |

Option A — first site in source order, parse time (ParseWarn channel):
- fires for every written `unique`/`unique0` qualifier the preprocessor keeps, reached or not (c13b, c14, c17); iverilog fires per ELABORATED site (c17 yes, c13b/c14 no); verilator only on executed overlap.
- cost: hdl-parser `ParseWarnKind` + latch in `parse_unique_priority` + `frontend.rs` arm + one `msgcodes!` row + doc15 + manual 007 + docs (G6); no AST / .vu / .velab / SimIr / format_version change; byte-identical where no `unique`/`unique0` qualifier (G3); staged prints at vcmp only (W2004 / W1017 precedent, S/g/c30_staged); tests: 12 FAIL in 2 files (G4).
- determinism: argv/expanded-buffer token order (c43), no hash iteration.

Option B — first elaborated (reached) site, elaborate time:
- needs an AST carrier: `unique0` and `unique`/`priority` with an explicit default/else leave no trace after parsing (G3, .velab byte-identical to plain apart from the 32-byte upstream digest). Precedent `TopItem::InsideNameUse` => `crates/hdl-ast/tests/schema_hash.rs` re-pin, all `.vu` stale (exit 2 at the schema gate), format_version unchanged.
- "reached" = elaborate's lowering walk: module map order, instance DFS, package / class lowering, on-demand subroutine lowering (`crates/elaborate/src/driver.rs:640-660`). Matches iverilog on c13b/c14; c17 (uncalled function) depends on whether elaborate lowers uncalled subroutines (NOT measured).
- prints at velab in the staged flow; c43 order becomes instance order, not argv order.

Option C — first executed site (engine): needs a runtime side table / new severity path; verilator-like timing; no oracle prints a "not checked" notice at run time. Not costed.

Qualifiers (IEEE, G1): include `unique` and `unique0` on case/casez/casex/`case inside` and on `if` (§12.4.2 "A unique-if and unique0-if assert that there is no overlap"; verilator reports both, c05/c06; iverilog has no `if` precedent — parse error). Exclude `priority`/`priority0` (§12.4.2/§12.5.3 define no overlap violation for priority; both oracles silent on c07/c08).

Severity / code facts:
- `Note`: printed token `note`; manual 007 defines Note as "Follow-on detail for the error above it. Carries that error's code." (doc edit); doc15 letter set is E/W/I/F (:26-27), no `N`; existing notes print `note[VITA-E3009]` (letter ≠ token already happens). Registering default severity Note passes bijection (`token()` vs `(Note)` lowercased) but needs a letter decision; registering `I…` and emitting Note needs a row in manual 007 / doc15 "emitted at a severity other than default" tables (:618 / :92-96).
- `Info`: `info[VITA-I2021]`, same `notes=` bucket, `-Wno-` suppressible, not promotable, exit unchanged; consistent letter. Precedent Info codes are user `$info` only.
- `Warning`: counts in `warnings=`; `-Werror` would turn every design with a qualifier (ibex) into exit 1 (c41 `-Werror` path promotes Warnings only).
- Measured gates on the probe (Note severity, c41): `-q` keeps it (stdout 0 bytes, note on stderr); `-Wno-I2021` drops it and epilogue `notes=0`; `-Werror` does not promote it (c47 rc0); `-Werror=I2021` accepted, no effect (rc0).
- Band: emitted by the parse stage -> 2xxx; next free 2021 (2004-2020 reserved in doc15 Appendix A). 4032 if placed beside W4031 (4xxx = runtime stage per doc15 "Number bands").

Location: iverilog prints `file:line` (qualifier keyword line, no column) per site; `loc_from_span` gives `file:line:col` of the qualifier token, through macros (c46: call site) and per file (c43). A location-less note (like W1017) would make "first such site" moot and drop the anchor ER §2.7 asks for ("Put the identifier and discriminating rule into a new loud message"); with a location the line also states what is meant by "first".

Proposed text (house style: what is and is not done, IEEE clause, other tools; W2004 is the model):
`t.sv:6:5: note[VITA-I2021] I-PARSE-UNIQUE-UNCHECKED: vita does not check a `unique` / `unique0` case or if for more than one matching item or condition (IEEE 1800-2017 §12.4.2, §12.5.3); the first matching item or condition runs and nothing is reported. Said once per run, at the first such statement`
(mnemonic/number illustrative; the probe used a different sentence). Wording constraint from G4: the 12 broken pins key on the `[VITA-` token, not on the text; no measured test keys on the words `unique` / `case` in a note.

## G6 ROADMAP §2 / PROBE_CATALOG rows touched or made stale

- `grep -nE "parse_unique_priority|unique0?\b" docs/ROADMAP.md`: §2 has NO row naming `parse_unique_priority`; §2 hits are 🆕 AA (:137), 🆕 AB (:138), :379 (a self-timed `always` split mentioning a `unique` miss) — none touched by a parse-time note.
- Rows this slice closes / edits: ROADMAP:548 §3.b `unique-overlap-note` (delete on close); ROADMAP:649 §5.2 row 1 (delete, renumber; LOOPROMPT NEXT row 1 is its index); ROADMAP:714 §8 non-goal text "unique / priority multiple-match checking (§3.b `unique-overlap-note` announces it)" -> must name the new code instead of a closed row id; ROADMAP Summary :18 `| §3.b | … | 128 | 111 | 17 | …` and total :27 `459 | 272 | 187` recount.
- Same-function rows NOT touched but adjacent: :547 `unique-if-chain` (BLOCKED), :549 `unique-const-fn` (§5.2 row 2), :550 `unique-pkg-closure`, :551 `unique-if-text` (W4031 wording on `if`). A parse-time latch in `parse_unique_priority` before `self.bump()` does not change the desugar those rows describe.
- `docs/PROBE_CATALOG.md`: no row names `parse_unique_priority`/`unique0`; :103 (§4.5.585 grounding) mentions "a `unique` miss in such a task reports at t0 through a continuous assign" — not touched.
- Docs that become stale when the note lands (text measured): `docs/manual/006_limitations.md:209-213` "The multi-match uniqueness check is a documented cut, and nothing announces it at run time yet (Icarus Verilog says `sorry: …`; ROADMAP §3.b `unique-overlap-note`)"; `docs/manual/003_language-reference.md:763` ("Multi-match checking is a documented cut — the lowered cascade is first-match-wins, so an overlap is unobservable."); `docs/manual/007_error-codes.md` Severity table Note row ("Follow-on detail for the error above it. Carries that error's code.") if Note severity is used, plus the 2xxx (:368-382) or 4xxx band table row; `docs/preview/15-error-code-reference.md` new body entry (+ "Number bands"/governance letter list :26-27 if a new `N` letter); `docs/preview/01-goals-and-scope.md:136` ("Multi-match is not checked …"); `docs/preview/13-diagnostics-and-logging.md:838` table (W4031 row; a new row if listed there); `docs/preview/hdl-reference/systemverilog/03-procedural.md:90-92` says "A warning if two or more branches match at once." (generic SV reference, reads as vita behaviour); README.md:112 lists the qualifiers (no overlap claim); CHANGELOG `[Unreleased]`.

## G7 lane table (ER §10.2) — for the parse-time option (A)

Shared function edited/routed into: `parse_unique_priority` (one caller, `stmt.rs:165`), `parse_with_warnings` / `ParseWarn` (one product consumer `frontend.rs:338`), `StderrSink` / `GatedSink` (unchanged code; new event). Emission precedes elaborate and backend choice (`frontend.rs:590` then elaborate).

| lane | path that prints | PRE (S/pre/vita, sep bins) | probe (scratch, NOT a proposal) | status |
|---|---|---|---|---|
| native (default) | frontend -> StderrSink | no note on any G1 cell | note ×1, c41/c48 | measured |
| interp (`--backend interp`) | same frontend | — | c41: stdout+stderr `cmp` identical to native, rc 0/0 | measured |
| vm (`--backend vm`) | same | — | c41: identical to native | measured |
| staged multicall (`vita vcmp/velab/vrun`) | vcmp = `pipeline.rs:519` frontend | W2004 proxy prints at vcmp only (c30) | c41: note at vcmp ×1; velab `errors=0 warnings=0 notes=0`; vrun none | measured |
| staged separate-bins (`vcmp`/`velab`/`vrun`) | same `run_vcmp` | c30 (W2004 proxy) | c48: vcmp I2021=1, velab 0, vrun 0 (probe sep md5 vcmp=1944c4df) | measured |
| JIT (`--features jit`, `VITA_JIT=1`) | same frontend | — | c48: identical to default (stdout+stderr); `VITA_JIT_STATS`: `JITBODY templates_compiled=0 refused=0 activations=0` (JIT ran nothing on this design; note is pre-engine) | measured |
| product (`--no-default-features`) | same frontend | — | c48: identical to default; `--backend interp` -> rc3 `error[VITA-E0001]: '--backend' takes only 'native' in this build …` | measured |
| `-q` / `--log` / `-Wno-` / `-Werror` | GatedSink + StderrSink | — | c41/c47: -q keeps note; -Wno-I2021 drops it (`notes=0`); -Werror and -Werror=I2021 do not promote (rc0) | measured (`--log` tee not run; same `tee()` as every diagnostic, lib.rs:271-275) |
| `--obs-dir` run.json | `frontend.rs:907-908` records errors/warnings only | — | c41: run.json differs only in wall_s/elab_s/sim_s; results.jsonl identical | measured |
| `hdl_parser::parse` callers (44; sim-engine tests etc.) | warnings dropped at `api.rs:14` | — | opted out by construction; probe gate: 9020/9032 pass, 12 FAIL all in two cli files (G4) | opted out |
| corpus (ibex, the only workload with a qualifier) | one-shot | c40 PRE: DIGEST=13b2ddfcd551ba2f rc0 `notes=0` | c40 probe: stdout `cmp` identical, rc0, stderr +1 line first, `notes=1` | measured |
| non-unique designs | — | examples ×4 (c49), priority-only c22 | stdout, stderr, rc and 4 VCDs `cmp` identical PRE vs probe; product stderr identical | measured |

Unmeasured cells: work-library multi-`vcmp` (`--work` + `velab -L`): mechanism = one note per vcmp invocation; not run. Option B (elaborate-time) lanes: none measured (no probe built).

Probe artefacts: S/probe_src (git archive HEAD + patch), S/probe_target (CARGO_TARGET_DIR, debug + release), S/probe_bin/{vita_default md5 9d6736b5…, vita_jit md5 2c9cbee7…, vita_product md5 67f32dbd…, sep/}. Logs: S/probe_nextest.log, S/probe_rel.log, S/probe_jit.log, S/probe_prod.log, S/probe_sep.log.

## Open decisions (facts behind each option)

1. Which sites count: A parse-time first written site (fires for c13b `--top`, c14 untaken generate, c17 uncalled fn; iverilog: c17 yes, c13b/c14 no) vs B first elaborated site (needs AST carrier for unique0 / unique+default — G3 byte-identity proof shows no trace; schema_hash re-pin, .vu stale, prints at velab). A has a measured probe and lane table; B has none.
2. Severity: Note (manual 007 Note row + doc15 letter set E/W/I/F must change or a default-vs-emitted row added) vs Info (no doc model change; token `info`) vs Warning (breaks `-Werror` users on every design with a qualifier, e.g. ibex).
3. Code / band: 2xxx (parse stage; next free 2021 per doc15 Appendix A) vs 4xxx beside W4031 (next free 4032; band = runtime stage). Mnemonic letter must match default severity (bijection checks number + severity per entry).
4. Location: with `file:line:col` of the qualifier token (probe; resolves macros c46, files c43) vs location-less like W1017.
5. Staged semantics: parse-time note prints at vcmp only (W2004/W1017 precedent) -> test util `diags()` (`crates/cli/tests/always_comb_t0_util/mod.rs:30-46`) must filter it or the 12 pins + `staged_matches` diverge; B would print at velab.
6. Docs: manual 006:209-213, manual 003:763, manual 007 (severity table + band row), doc15 entry, preview 01:136, preview 13:838, preview hdl-reference 03-procedural.md:90-92, CHANGELOG, ROADMAP :548/:649/:714 + Summary, LOOPROMPT NEXT row 1.

