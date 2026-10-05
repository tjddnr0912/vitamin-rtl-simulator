# r1 soundness lens — §4.5.586 unique-overlap-note
status: DONE (round 1); verdict PASS: no BLOCKING; 3 non-blocking NEW
## Q1 qualifier consumers census — DONE (grep)
Kw::Unique/Unique0 src sites (all crates): stmt.rs:165 (stmt dispatch -> parse_unique_priority, records), assertions.rs:479/485 (inside parse_unique_priority),
lib.rs:1224-1225 member_ident (dot-position name e.g. q.unique(); not a qualifier; no record = correct). Lexer lib.rs:689-690 keyword table.
Backtracks (pos=save): expr_primary.rs:207-216 ($bits type arg), functask.rs:412-417 (const ref), type_params.rs:440-499/897-921 (type param value). None spans parse_stmt.
'before = self.pos' sites (~37) are progress guards, not rewinds. No errors/warnings truncate anywhere. One Parser::new (api.rs:28).
## Q2 warnings writers / readers — DONE (grep)
writers: lib.rs:1160 warn_select_base, lib.rs:1181 record_unique_qualifier; mover api.rs:40. readers of ParseWarnKind: cli/src/frontend.rs match only (exhaustive, no `_`). parse() drops warnings (api.rs:15).
## Q3 parse-call count — DONE (grep)
parse_with_warnings product caller: frontend.rs:338 only; frontend_pp_to_unit_mapped <- frontend_sources_mapped(298) <- run_vita_str_gated(frontend.rs:615, one-shot) and run_vcmp_gated(pipeline.rs:519, vcmp; file loop at 494 only reads files).
hdl_parser::parse product callers: none (backend.rs:1565 is inside mod tests @1242). velab/vrun: no frontend call.
## Q4 emission point — cells c01-c07 (PRE/POST)
pre-emit returns: pp errors, lex errors, parse errors (frontend.rs:372-384). post-emit: `let Some(unit) else` "no design units found" E2002 (frontend.rs:404-415), then elaborate.
c02 (CU-scope function only): POST prints I2021 THEN `error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: no design units found in source`, rc=1; PRE prints the E2002 only. -> contradicts "a failed parse prints exactly what it printed before" / "a design that fails to parse does not print it" (E2002 is the parse code). NEW, non-blocking.
c01 (elab error E3003): POST I2021 then E3003 rc=1. c03 class-only / c07 package-only: I2021 then E3009 no top. c04 constraint `unique{}`: parse error both, no I2021 (no over-detect). c05 W2004 before unique: I2021 printed (product ok). c06 `unique begin`: parse error, no I2021.
## Q5 util filter — DONE
users: always_comb_t0_after_settle.rs (16 diags calls), always_comb_t0_splits.rs (12); both have unique designs; neither is about I2021 (pinned in unique_overlap_note.rs with raw stderr via its own notes()). staged_matches still compares the whole vrun Run (out+diags+code) to one-shot: real comparison. No hidden assertion found.
## Q6 test teeth — mutant built (own target, git-archive fc1fc07e + slice.patch)
MUT latch-any: lib.rs record_unique_qualifier `.any(|w| w.kind == UniqueOverlapUnchecked)` -> `.any(|_| true)`: unique_overlap_note 14/14 PASS (SURVIVED, scoped). On c05 (W2004 select at line 4 before unique at line 5) mutant prints NO I2021; POST prints c05_w2004_first.sv:5:5 I2021. Only W2004 in test file is line 523, after the unique at 522. --workspace re-check: SURVIVED (9046/9046).
emit-after-no-design-units: equivalent on all tests (no CU-only source in tests); product differs on c02.
## Q7 doc claims
OK (006 iverilog rule, c12): `iverilog -s top` prints sorry only at line 4 (uncalled function); uninstantiated module (line 1) only without -s; untaken generate (line 5) never. vita POST: c12:1:56 (uninstantiated module) as documented.
Q1 extra: c11 checker/program -> loud parse errors both (no silent skip); skip helpers cover.rs:396 / classes.rs:534 / expr.rs:606 are bins/recovery/brace groups (c04: constraint `unique{}` is a loud error).
UNVERIFIED: 'Six of the 71 ... produced by no code path' (pre-existing claim, denominator-only edit; crude MsgCode:: grep gives 10, misses non-literal producers).
OK: counts 71 / 31 Warning / 3 Info (code.rs grep); no stale 68/69/70 count left (grep md+rs); doc15 example -> m.sv:4:8 + text exact; iverilog `m.sv:4: vvp.tgt sorry: ...` exact; artifacts .vu/.velab byte-identical PRE vs POST (unique c09 + control c10); PRE .vu -> POST velab/vrun ok; c03/c07 unreached print.
REFUTED (comment frontend.rs:386-389): 'iverilog likewise prints its sorry only for a design it compiles': c01/c08 elab errors -> iverilog prints errors only, no sorry; vita POST prints I2021 then E3003/E3010.
REFUTED (doc15/007/006/CHANGELOG + comment): 'a failed parse prints exactly what it printed before' / 'a design that fails to parse does not print it': c02 -> I2021 + E2002 'no design units found in source'.
## Findings (most severe first; all NEW = absent on PRE, all non-blocking)
F1 emit precedes the 'no design units' return (frontend.rs I2021 block sits before `let Some(mut unit) = unit else`). c02 (CU-scope function only): POST I2021 + E2002 'no design units found in source' rc=1 (vita and vcmp); PRE E2002 only. Refutes comment 'a failed parse prints exactly what it printed before' and doc15/007 'a design that fails to parse does not print it'. Unpinned (moving the emit after that return is equivalent on all 14 tests).
F2 comment analogy 'iverilog likewise prints its sorry only for a design it compiles' holds for parse failures only: c01/c08 elab errors -> iverilog no sorry; vita I2021 + E3003/E3010.
F3 teeth: MUT latch-any survives unique_overlap_note 14/14; c05 shows it drops I2021 (W2004 before first unique). Workspace re-check: `cargo nextest run --workspace --locked --no-fail-fast` on the mutant copy -> rc=0, 'Summary [41.069s] 9046 tests run: 9046 passed, 15 skipped' => SURVIVED --workspace (log r1/sound/mut_latch/ws.log).

wall_s: 712

# Round 2 (delta, POST2)
status: IN PROGRESS
binaries: POST2 vita 9d5134826d5c2e971c6ca7bf1e841a86, sep vcmp 21c9036c velab 74421968 vita f5b15622 vrun ee826236; slice2.patch 71224f4b; worktree tracked diff == slice2 (14 tracked + 1 untracked test).
Q-a returns: frontend.rs POST2 parse gate return 384, no-unit return 397, emit 404-417, no return after 404 in the fn (Some at 496) -> every parse-stage success reaches the emit.
Q-b cells POST vs POST2: only c02 differs; c02 PRE==POST2 byte-exact for vita and vcmp (F1 fixed). c13 pp error, c14 unterminated string (E1013): PRE==POST2 byte-exact (vita, vcmp). c01/c08 elab errors still print (now documented).
Q-c mutants (own target, slice2 copy r1/sound/mut2): MA latch-any -> killed by a_select_base_warning_before_the_first_unique_does_not_take_its_place (15/16); MB emit above no-unit return -> killed by a_source_with_no_design_unit_prints_only_its_error_on_vita_and_vcmp (15/16); restore clean.
Q-d failure paths after emit (print I2021, consistent with 'parses then fails later'): E2001 dup unit (elaborate driver.rs:722/735/824) c16; -Werror-promoted W2004/W1017 c05 (vcmp: no artifact).
Q-e priority0: POST2 priority0 no-match -> no W4031, no I2021; priority -> W4031, no I2021. iverilog 'syntax error' and verilator 5.052 'syntax error, unexpected case' on 'priority0 case'; verilator accepts 'priority case'.
Findings r2: R2-1 (nit, NEW, same root as F1): doc15/006 'a source with no design unit ... log exactly as before' means vita's E2002 'no design units found'; c03 class-only (no IEEE design unit) prints I2021 + E3009. R2-2 (nit, NEW): doc15 'and neither does priority0' ambiguous. R2-3 (pre-existing, not in delta): assertions.rs '§12.4.2: the 0 variants' files priority0 under IEEE, contradicting the new 'non-standard priority0' doc 30 lines above.
verdict r2: PASS. wall_s r2: 217
