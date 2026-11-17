(goals) (how a goal doc is written, and when the plan is re-read):
* a goal is a set of designs of the real-design set that RUN when it is reached (`docs/ROADMAP.md`, Direction, The end); goals run in ladder order (Order rules O6), and the owner decides the ladder (O9).
* one doc per goal, `roadmap/goals/<G-ID>.md`, linked once from the goal ladder; line 1 is `(goal <G-ID> · <position> · <state>) (<what runs>):`, its title the ladder line's and its state the ladder's word (`reached`, `current`, `next`, `later`, `proposed`).
* then one bullet per label, in this order (forward plan 4.1, W/program/roadmap-plan-2026-11-17/PLAN.txt):
  * `reach:` the designs and the test that says the goal is reached (its corpus row id), and the scoreboard after it.
  * `why here:` one clause on why the goal stands at its place on the ladder; `status:` decided or proposed, by whom and when, and the ladder's word.
  * `chain:` the rows in run order, as a table `| # | id | kind | state | why |`: kind one of record, prerequisite, design, corpus; state one of the six state words (roadmap/README.md); why one clause on why the row stands there; a row with no id yet reads `not filed` with its one-line purpose.
  * `records:` what must exist before the chain's readers, and which row builds it.
  * `unowned:` `none`, or the blocker that has no row and what will own it.
  * `risk:` the largest risk; `signal:` the cheapest early signal that would break the estimate.
  * `size:` the estimate (listed rows → rows run, batches, weeks), its date, and what would change it.
  * `evidence:` pointers to the measured views under W, dated, never copies of their numbers.
  * `decisions:` one dated line per owner or main decision that changed the chain.
  * `batches:` which rows landed in which batch, with the §4.5.N of its landing.
* a proposed goal's doc is a stub: `reach:`, `known members:`, `status: proposed` and `re-censused at entry`; it is filled from its entry census when the goal becomes current (migration step S8), and its rows move onto its chain then.
* a goal doc states direction, made by reasoning (owner, 2026-11-17): nothing in it is measured per item, and a row's detail is re-measured when the row becomes a slice; a chain row's own doc keeps its record.
* checkpoints, when the plan itself is re-read and the one question asked (forward plan part 5); the owner is asked at each one (owner Q2 (a), 2026-11-17):
  * C1 every park (main, no agent): does the park name a prerequisite row or DORMANT, and is the goal doc's chain changed in the same commit?
  * C2 every second batch boundary (main, no agent; every fourth coincides with the D14 structure review): has a chain passed 1.5x its listed rows, is any row on a chain without a route, has a group's trigger fired, are State and ladder true?
  * C3 every goal reached (one plan-agent pass, read-only): did the chain cost what the goal doc estimated (rows listed against rows run, parks, batches); correct the next two goals' estimates by that ratio; is the next goal still the best designs per row?
  * C4 the first testbench run of a family on original source (row 31 for VeeR, row 47's pattgen page for OpenTitan): did "opens" become "runs"? If not, the divergence's group (K5 or K6) gets a chain position and the following goals' sizes are re-estimated.
  * C5 G-OT5 reached: the ladder below G-OT5 is re-decided with the owner from a fresh measured view (G-OT9, G-EH1D, G-OT19 and G-EL2 are proposals until then; owner Q3 (a), 2026-11-17).
