(goal G-EH1 · 1 · current) (VeeR EH1 `eh1_noassert` runs from original source):
* reach: VeeR EH1's `eh1_noassert` configuration runs from its original source under its first-party testbench with oracle agreement, as corpus row §5.2 31 (hello_world on unmodified RTL); the scoreboard after it reads originals running 2 / 23.
* why here: the shortest measured chain to any original design (two design rows), and its rows 24 and 29 are reused by EL2, `eh1_default` and every OpenTitan page.
* status: decided (owner, 2026-11-16 and 2026-11-17); current.
* before it: batch 5 landed on main on 2026-11-18 (§4.5.654: PARSE-1's provenance piece and Q-WREXT, each structure only); Q-MDRVMIX, built in batch 5, was parked on 2026-11-17 (owner, option A) and its commits pulled out (a33df0e0), so it is on this chain, behind the structural row Q-WEXTPOS.
* chain (run order; batch 6 = rows 1–3, batch 7 = rows 4–6, batch 8 = rows 7–8, Order rules O5 applied to the 8-row chain of 2026-11-17, re-read at batch 6's start; the owner's Q1 (a) cut was made for the 6-row chain):

| # | id | kind | state | why |
|---|---|---|---|---|
| 1 | Q-WEXTPOS | record | OPEN | Q-MDRVMIX's park (2026-11-17): the write view stands down on a provisional constant and carries each write's own position, so Q-MDRVMIX's re-plan reads both from the record (ER §5.1; Q-WEXTPOS.md) |
| 2 | Q-MDRVMIX | prerequisite | PARKED Q-WEXTPOS | row 24 waits on it (owner option 2, 2026-11-16); built in batch 5 and pulled out at its boundary, it returns by a fresh plan once Q-WEXTPOS lands (Q-MDRVMIX.md, the park and re-entry) |
| 3 | Q-LOSSYSPELL | record | OPEN | the design-level term (which lossy constant producers a design spells) has three readers, row 24, 29's page half and R6.1 (Q-VALRULES), and no record holds it (O2 (a)); owner Q2 (a) of the roadmap review, 2026-11-17 |
| 4 | 24 (EH1 cut) | design | PARKED Q-LOSSYSPELL, Q-MDRVMIX | opened narrowed to the tokens EH1's original source uses (owner, 2026-11-16); it waits again on Q-MDRVMIX, parked 2026-11-17 behind Q-WEXTPOS (Q-WREXT landed, §4.5.654), and it re-enters by a full re-plan that reads the term (24.md) |
| 5 | Q-OVRXPLANE (typed lane) | prerequisite | OPEN | 29's page half waits on it (29.md); its typed-target lane reads Q-DECLTYPE's S12 record (landed, §4.5.635; its list line left with the landing) and row 48's D4 (delivered), while its untyped-keyword targets stay behind S1v-V, DORMANT in K2 (Q-OVRXPLANE.md) |
| 6 | Q-LITLANE | prerequisite | OPEN | `const_eval_u32`'s literal lane is the producer of 29's r1, and the page half waits on it (29.md, Q-LITLANE.md) |
| 7 | 29 (page half) | design | PARKED Q-LOSSYSPELL, Q-OVRXPLANE, Q-LITLANE | holds EH1's lines on element selects nested in a write target; reads the term in place of S1v-V (owner O2, 2026-11-15); its full form stays PARKED behind Q-IDXSIGNCTX and Q-IDXVALUE, off this chain (29.md) |
| 8 | 31 | corpus | WAITS 24, 29 | the reach test: the first testbench run on unmodified EH1 RTL at HEAD; row 26's testbench site stays precomputed strings in the first-party testbench (31.md) |

* records: Q-WEXTPOS, row 1 (the write view's stand-down and each write's position; reader Q-MDRVMIX); Q-LOSSYSPELL, row 3 (the term; readers 24, 29's page half, R6.1); Q-WREXT's write-extent view (landed, batch 5, §4.5.654); BIND-1's home (landed, §4.5.640) under Q-OVRXPLANE; LitOrigin, PARSE-1's provenance piece (landed, batch 5, §4.5.654), the fact Q-LOSSYSPELL can be computed over.
* unowned: none named: the 2026-11-16 candidate pass names row 24 and then 29's page half as EH1's shortest chain (W/program/next-2026-11-16/PLAN.txt).
* risk: Q-LOSSYSPELL is by its nature a statement about what a design SPELLS, and "a census by spelling" is one of the two classes premise judges fail on.
  * If the term cannot be stated over recorded facts (LitOrigin::Source literals, declaration facts) finely enough that EH1 itself satisfies it (EH1 spells unbased `'x` fills in mem_lib.sv and no unsized based x / z literal, state.txt 2026-11-16), the chain falls back to the producers themselves (Q-UNSZXPAD first, then 76 behind 8 and S1v-V): at least 3 more rows, some of them in the dormant cluster K2.
* risk (others): Q-LITLANE reads VAL-2's head-note lane table, built with VAL-2's piece P-a; VAL-2's entry is not built, and whether the lane needs it is the row's grounding (VAL-2.md, Q-LITLANE.md); where an override's x plane is lost is NOT_MEASURED (Q-OVRXPLANE.md); 29's page half took 3 + 3 lens rounds; 31 is the first testbench run on unmodified RTL at HEAD.
* signal: Q-LOSSYSPELL's plan pass: before any build it must answer "does `eh1_noassert`'s original source satisfy the term as rows 24 and 29 read it"; a no there re-orders the goal on day one. Second signal: row 24's premise judge, within two tries.
* size: ESTIMATE, 2026-11-17, made for the 6-row chain: 6 listed rows → 8 to 10 run, 2 batches (cut before 29, O5), 2 to 3 weeks; the 8-row chain after Q-MDRVMIX's park is 3 batches (O5) and is not re-estimated. What changes it: the two signals above.
* evidence: W/program/next-2026-11-16/ (PLAN.txt, designs.tsv, blockers.tsv; the candidate pass, 2026-11-16); W/r24e/ground/REPORT.txt (row 24's grounding, EH1's lines); W/r24e/premise/REPORT.txt (row 24's premise judge 1); W/program/roadmap-review-2026-11-17/FIXLIST.txt (2026-11-17); W/program/roadmap-plan-2026-11-17/PLAN.txt 1.2–1.3 (the estimate, 2026-11-17).
* decisions:
  * 2026-11-16, owner: EH1's original source first, row 24 opened narrowed to EH1's tokens (the candidate pass's option 1).
  * 2026-11-16, owner: the multidriver rule for partial and mixed writes goes before row 24 (option 2): Q-WREXT, then Q-MDRVMIX.
  * 2026-11-17, owner: Q-MDRVMIX's rule Y is an error, IEEE first.
  * 2026-11-17, owner (roadmap review, Q1 (a) and Q2 (a)): batch 5 closes at Q-MDRVMIX; Q-LOSSYSPELL is its own structural row before row 24.
  * 2026-11-17, owner (forward plan, Q1 (a) and Q2 (a)): the chain is cut before row 29; main opens the next chain row without asking.
  * 2026-11-17, owner (option A): Q-MDRVMIX is parked and its two commits pulled out of batch 5; it returns behind a structural row that gives the write view each write's own position and its stand-down on a provisional constant (Q-WEXTPOS).
* batches: no chain row has landed yet; batch 5 (PARSE-1's provenance piece, Q-WREXT), the work before row 1, landed 2026-11-18 (§4.5.654); batch 6 = rows 1–3, batch 7 = rows 4–6 and batch 8 = rows 7–8, planned (O5, re-read at batch 6's start).
