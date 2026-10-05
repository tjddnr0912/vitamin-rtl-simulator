# §4.5.591 lens DIFFERENTIAL round 2 (delta) — REPORT (final)
round: 2 · verdict: PASS (no correct->loud vs PRE, no new silent-wrong vs PRE, no staged divergence) · 1 MAJOR doc wording, 1 MINOR ladder trade
binaries: PRE ac965d29… · POST_a 732fb809… · POST_b $S/s591/post_b/vita 1d520f3399f0cd76ed243b6a24cd86cb (staged post_b/sep) · diff b63db492…
harness: $S/s591/r2/diff/run_r2.sh (pre, posta, postb, postbstg, ivl, vl, s2v; -I. everywhere); cells $S/s591/r2/diff/cells (12 new: R01-R06, R08-R11, R13, R14)

## r1 43 cells on POST_b
staged==one-shot 43/43; POST_a->POST_b moved 1 (D24: loud -> `d24 Z=3 Y=5` = PRE, 3 oracles `Z=3 Y=3`, pinned KNOWN-WRONG); PRE->POST_b movers 4 (D40, D41 silent->loud = ivl/s2v refuse; D42, D54 silent->correct).

## 12 position-rule cells (base: pa P=3, pb [64:0] P=65'h1_..._0009)
| cell | item/import between | PRE | POST_a | POST_b (= staged) | ivl | vl | s2v |
|---|---|---|---|---|---|---|---|
| R01 | `define'd `localparam int A = P;` | A=3 P=3 | E3009/E3010 | A=3 P=3 | A=3 P=3 | A=3 P=3 | A=3 P=3 |
| R02 | second import from a macro | A=3 P=3 | loud | A=3 P=3 | = | = | = |
| R03 | `include'd item | A=3 P=3 | loud (`R03.svh:1:20`) | A=3 P=3 | = | = | = |
| R04 | `include'd import | A=3 P=3 | loud | A=3 P=3 | = | = | = |
| R05 | generate-if on P | g3 P=3 | loud | g3 P=3 | = | = | = |
| R06 | initial reading P | 3/3 | loud | 3/3 | = | = | = |
| R08 | header `import pa::*, pb::*;` | P=3 | E3010 | E3010 | "Ambiguous use of 'P'. It is exported by both 'pa' and by 'pb'." | P=3 | "identifier "P" ambiguously refers to the definitions in any of pa, pb" |
| R09 | header param W=P, body import | W=3 P=3 | loud | W=3 P=3 | = | = | = |
| R10 | port `[P-1:0]`, body import | b=3 P=3 | loud | b=3 P=3 | = | = | = |
| R11 | three wildcards, ref after first | A=3 P=3 | loud | A=3 P=3 | = | = | = |
| R13 | interface | A=3 P=3 | loud | A=3 P=3 | = | = | = |
| R14 | package: explicit `px::Q` between two wildcards | Z=9 | E3009 | E3009 | "Ambiguous use of 'P'..." | Z=3 | "ambiguously refers" |
10 cells = POST_a correct->loud (same root class as r1 soundness F1), all = PRE = 3 oracles on POST_b. R08/R14 silent->loud on designs ivl+s2v refuse.

## findings
- G1 MAJOR (doc, new): test header bullet "a package body keeps ONE wildcard-origin / explicit-import state, as a module does, so its ambiguities are seen (with the same position rule)" — across an item the package resets to PRE per-import state (later package wins: c07 `pr A=3 B=5`, D24 `d24 Z=3 Y=5`, c32 `pnw A=3 B=9`; 3 oracles first package) while the module lane answers the first package (c22, R01-R13 `A=3 P=3`). Same adjacency test, different non-adjacent arm: say so.
- G2 MINOR (ladder trade, new): package Fresh-across-item rule rests on n03 (pa::P = pb::P = 3): PRE `n03 A=3 B=3` is right only by value coincidence — same mechanism gives c07 `B=5` vs 3 oracles `B=3`. ER §7.3: a coincidence-correct cell is no control. POST_a was loud on c07/c32/D24 (ladder-better than PRE); POST_b returns them to silent-wrong (pinned). Not a regression vs PRE; coordinator to adopt/reject.
- residue unchanged: n04 class (item between, no reference) = PRE silent; D19/D20; F5 location.
