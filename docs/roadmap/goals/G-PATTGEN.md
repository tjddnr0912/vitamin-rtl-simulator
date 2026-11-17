(goal G-PATTGEN · 2 · next) (OpenTitan pattgen runs from original source):
* reach: OpenTitan pattgen runs from its original source under its first-party TL-UL testbench with oracle agreement, as the pattgen page of corpus row §5.2 47; the scoreboard after it reads originals running 3 / 23.
* reach (beside it): the goal removes the four blockers that stand on all 19 OpenTitan originals (rows 23, 24, 33, 46).
* why here: no OpenTitan page opens without all four, and pattgen is the page that needs nothing else; low designs per row (one for eight), the highest reuse of any chain.
* status: decided (owner, 2026-11-17, roadmap review Q3 (a)); next.
* chain (the owner's order; inside it a prerequisite goes directly before its reader):

| # | id | kind | state | why |
|---|---|---|---|---|
| 1 | Q-TYPEOVERVAL | prerequisite | OPEN | it holds row 33, PARKED 2026-11-04 behind it, and through 33 R08 on every OpenTitan page; in the parser, census first (Q-TYPEOVERVAL.md) |
| 2 | 33 | design | PARKED Q-TYPEOVERVAL | R08: the lines each page holds come from ONE function body, prim_assert's static-assert frame, elaborated though uncalled (33.md, W/program/next-2026-11-16/blockers.tsv) |
| 3 | Q-PKGDUPNAME | prerequisite | OPEN when G-PATTGEN is current (O3) | row 23's RE-e: §2 duplicate-value-name-accepted's package-body pairs, which row 23 would route onto (O3; owner Q4 (a) of the roadmap review, 2026-11-17) |
| 4 | 23 | design | PARKED Q-PKGDUPNAME | the parser's shift and power fold: `tlul_pkg`'s member widths stop all 19 OpenTitan originals; its reader declines on a Parser-origin leaf (PARSE-1's provenance piece) (23.md) |
| 5 | Q-NETSHAPE (first commit) | record | OPEN | its declaration-side first commit records who wrote `DataType.range` (in place, a named typedef with its declaring scope, or parser-folded), read by 46 and later 71 (structure review 2's C, Q-NETSHAPE.md); a SchemaHash bump, so it starts a batch alone (O5); written under Q-NETSHAPE's id, not filed as a row of its own; the rest of Q-NETSHAPE goes to G-OT9 |
| 6 | Q-RTNTEXT | record | OPEN | the F25 caller-scope answer row 46 reads (owner, 2026-11-12, option (B)); its slot line's "after 68 + 23" is the record-first order's: 68 is DORMANT (K2) and 23 stands earlier on this chain (Q-RTNTEXT.md) |
| 7 | 46 | design | PARKED 23, Q-RTNTEXT, Q-NETSHAPE | it re-enters whole, not by page forms; its waits, re-read 2026-11-17, are row 23, Q-RTNTEXT and Q-NETSHAPE's first commit (46.md) |
| 8 | 24 (OT cut) | design | WAITS 24 (EH1 cut) | `tl_i.a_address[RegAw-1:2]` (roadmap review A8), on G-EH1's row 24 |
| 9 | 47 (pattgen page) | corpus | WAITS 33, 23, 46, 24 | the reach test: the testbench run that makes pattgen RUN, the first OpenTitan simulation on original source |

* records: PARSE-1's provenance piece (built, batch 5): row 23's reader declines on a Parser-origin leaf; Q-NETSHAPE's first commit, row 5 (who wrote `DataType.range`; readers 46, later 71), its own batch (O5); Q-RTNTEXT, row 6, a structural row already filed, before 46.
* unowned: none named; Q-TYPEOVERVAL's grounding may name more producer classes than one (the signal below).
* risk: row 33 parked after four lens rounds, each on a new instance of "a source the admission cannot see", and Q-TYPEOVERVAL is unstarted, in the parser, "census first".
* signal: Q-TYPEOVERVAL's grounding: if it names more than one producer class (a typedef seen through a wildcard, `$unit`, a type parameter, an enum label), 33 is two or more prerequisite rows away, not one.
* question for row 33's re-plan, not a plan item (UNVERIFIED, reasoning only): whether R08 can leave the pages by a route that does not open `'{default: v}` in a frame at all (the body is never called; §2-R unused-pkg-fn-framed is the neighbouring entry); it trades a refusal for an unchecked static assert, so it needs the owner if the plan agent proposes it.
* size: ESTIMATE, 2026-11-17: 9 listed positions → 11 to 14 run, 3 batches, 4 to 5 weeks. What changes it: the signal above; whether 24's OpenTitan cut is the EH1 cut's mechanism again (then it is half a row); whether 43 merges into 46 (43.md: "if row 46 takes the scoped spelling, rows 43 and 46 are one slice").
* evidence: W/program/next-2026-11-16/ (PLAN.txt, blockers.tsv, designs.tsv; the candidate pass, 2026-11-16); W/review/structure-2026-11-16/REPORT.txt (structure review 2, C); W/r23/plan/PLAN.txt (row 23's re-entry plan and RE-e, 2026-11-16); W/program/roadmap-review-2026-11-17/FIXLIST.txt (A8); W/program/roadmap-plan-2026-11-17/PLAN.txt 1.3 (the estimate, 2026-11-17).
* decisions:
  * 2026-11-12, owner: row 46's F25 is a new structural row, option (B): Q-RTNTEXT.
  * 2026-11-16, owner: row 23 parked; its prerequisite RE-e not opened then, the row opening no real design alone.
  * 2026-11-17, owner (roadmap review, Q3 (a) and Q4 (a)): pattgen is the goal after EH1, then the OpenTitan five; RE-e opens as Q-PKGDUPNAME when this goal starts.
* batches: none; planned at the goal's entry, Q-NETSHAPE's first commit starting a batch alone (O5).
