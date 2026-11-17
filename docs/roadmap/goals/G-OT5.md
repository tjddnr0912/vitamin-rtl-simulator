(goal G-OT5 · 3 · later) (uart, gpio, i2c and rv_timer run beside pattgen under TL-UL testbenches):
* reach: OpenTitan uart, gpio, i2c and rv_timer run from original source under first-party TL-UL testbenches with oracle agreement, beside pattgen, as the other four pages of corpus row §5.2 47; the scoreboard after it reads originals running 7 / 23.
* why here: three rows for four designs once G-PATTGEN's four blockers are gone, the best designs per row on the ladder.
* status: decided (owner, 2026-11-17, roadmap review Q3 (a)); later.
* chain (run order):

| # | id | kind | state | why |
|---|---|---|---|---|
| 1 | 35 | design | OPEN | R06, a replication inside an assignment pattern, refused at parse on uart, gpio, i2c and rv_timer (35.md); tier S with no `after` |
| 2 | 43 | design | OPEN | R01b, a package struct member width over a constant function call, on uart, gpio, i2c and rv_timer; "on the class": it reads the same provenance facts as 23 and 46, and merges into 46 if 46 takes the scoped spelling (43.md) |
| 3 | 37 | design | OPEN | R17 and R27, unpacked-array port actuals, on rv_timer (37.md); tier S with no `after` |
| 4 | 47 (other four pages) | corpus | WAITS 35, 43, 37 | the reach test, after G-PATTGEN's pattgen page |

* fillers: 35 and 37 are tier S with no `after`; by O1's filler clause they may be pulled forward into G-EH1 or G-PATTGEN whenever the chain's next row cannot start (they open nothing alone; they shorten this goal).
* records: none new (43 is "on the class": it reads the same provenance facts as 23 and 46).
* unowned: none named; 47's doc lists more rows from the older row-9 census (38, 39, 42, 69, 70, Q-VARBASEWR), which the 2026-11-16 measurement does not put on these four pages (W/program/next-2026-11-16/PLAN.txt); re-read at the goal's entry.
* risk: row 47 is the first time OpenTitan designs SIMULATE on original source; "opens" was measured, "runs" was not (W/program/next-2026-11-16/PLAN.txt, NOT MEASURED), and a testbench run can reach the engine-semantics group K6, which no goal chain has touched.
* signal: pattgen's testbench page at the end of G-PATTGEN: the same harness, one design earlier (checkpoint C4).
* size: ESTIMATE, 2026-11-17: 4 listed → 5 to 7 run, 1 to 2 batches, 1 to 2 weeks. What changes it: the first divergence under a testbench.
* evidence: W/program/next-2026-11-16/ (PLAN.txt, blockers.tsv, designs.tsv; the k6 and k8 twins, 2026-11-16); W/program/roadmap-plan-2026-11-17/PLAN.txt 1.3 (the estimate, 2026-11-17).
* decisions:
  * 2026-11-17, owner (roadmap review, Q3 (a)): after pattgen, 35 and 43 (uart, gpio, i2c), then 37 (rv_timer).
* batches: none.
