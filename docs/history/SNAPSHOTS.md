# Open-work snapshots

One row per loop iteration (appended in the docs step, after the ROADMAP Summary is recounted), so
the trend of the open work can be read across days. Columns are the Summary table's `open /
startable / blocked` per section; `total` sums them. Counts are rows or bullets, not slices. The
section keys: 2T = §2 start-order table, 2M = §2 defects by mechanism, 2N = §2-N, 3a / 3b / 3c = §3
numbered / small / intentionally loud, 0 = §0 promotion queue, 4 = SVA, 6 = G2 OBS (stages +
beside-track items, counted together), 5b = performance / hardening, 7 / 8 = conditional / non-goals.

| date | HEAD | slice | tests | fmt | 2T | 2M | 2N | 3a | 3b | 3c | 0 | 4 | 6 | 5b | 7 | 8 | total (open / startable / blocked) |
|---|---|---|---:|---:|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2026-09-18 | 772afe6 | §4.5.509 | 8004 | 32 | 27 / 6 / 21 | 116 / 76 / 40 | 7 / 0 / 7 | 24 / 19 / 5 | 91 / 79 / 12 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 14 / 13 / 1 | 18 / 9 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 335 / 211 / 124 |
| 2026-09-18 | ed702c4 | §4.5.510 | 8008 | 32 | 27 / 6 / 21 | 117 / 76 / 41 | 7 / 0 / 7 | 24 / 19 / 5 | 91 / 79 / 12 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 14 / 13 / 1 | 18 / 9 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 336 / 211 / 125 |
| 2026-09-18 | 1bfb086 | §4.5.511 | 8015 | 32 | 27 / 6 / 21 | 117 / 76 / 41 | 7 / 0 / 7 | 24 / 19 / 5 | 91 / 79 / 12 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 14 / 13 / 1 | 18 / 9 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 336 / 211 / 125 |
| 2026-09-18 | 65ed98e | §4.5.512 | 8018 | 32 | 27 / 6 / 21 | 117 / 76 / 41 | 7 / 0 / 7 | 24 / 19 / 5 | 93 / 80 / 13 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 15 / 14 / 1 | 18 / 9 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 339 / 213 / 126 |
| 2026-09-18 | 7e15625 | §4.5.513 | 8027 | 32 | 27 / 6 / 21 | 117 / 76 / 41 | 7 / 0 / 7 | 24 / 19 / 5 | 93 / 80 / 13 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 14 / 13 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 337 / 211 / 126 |

Notes per row go below, dated, only when a count moved for a reason the row cannot show (a recount,
a section restructure, a residue split).

- 2026-09-18: first row. The §2 mechanism and §3.b counts were recounted from the file (the previous
  Summary said 111 and 87; the bullets were 116 and 91). §4.5.509 closed one §2 mechanism bullet and
  re-filed its residues as one bullet, so 2M is unchanged by the slice itself.
| 2026-09-18 | d407854 | §4.5.514 | 8068 | 32 | 27 / 6 / 21 | 118 / 76 / 42 | 7 / 0 / 7 | 24 / 19 / 5 | 96 / 82 / 14 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 14 / 13 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 341 / 213 / 128 |
| 2026-09-19 | e6713aa | §4.5.515 | 8078 | 32 | 27 / 6 / 21 | 120 / 78 / 42 | 7 / 0 / 7 | 24 / 19 / 5 | 97 / 83 / 14 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 14 / 13 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 344 / 216 / 128 |
| 2026-09-19 | c32406e | §4.5.516 | 8111 | 32 | 27 / 6 / 21 | 121 / 79 / 42 | 7 / 0 / 7 | 24 / 19 / 5 | 97 / 83 / 14 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 14 / 13 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 345 / 217 / 128 |
| 2026-09-20 | 92d3be7 | §4.5.517 | 8146 | 32 | 27 / 6 / 21 | 121 / 79 / 42 | 7 / 0 / 7 | 24 / 19 / 5 | 96 / 82 / 14 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 14 / 13 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 344 / 216 / 128 |
| 2026-09-20 | d0ff5a2 | §4.5.518 | 8194 | 32 | 27 / 6 / 21 | 127 / 83 / 44 | 7 / 0 / 7 | 24 / 19 / 5 | 95 / 81 / 14 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 14 / 13 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 349 / 219 / 130 |
| 2026-09-21 | b32cf00 | §4.5.519–524 | 8306 | 33 | 26 / 0 / 26 | 140 / 92 / 48 | 7 / 0 / 7 | 24 / 19 / 5 | 98 / 84 / 14 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 14 / 13 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 364 / 225 / 139 |
| 2026-09-21 | c026a81 | §4.5.525 | 8376 | 33 | 26 / 0 / 26 | 142 / 87 / 55 | 7 / 0 / 7 | 24 / 19 / 5 | 102 / 87 / 15 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 14 / 13 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 370 / 223 / 147 |
| 2026-09-24 | 596f756 | §4.5.526 | 8411 | 34 | 26 / 0 / 26 | 137 / 82 / 55 | 7 / 0 / 7 | 24 / 19 / 5 | 104 / 89 / 15 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 16 / 15 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 369 / 222 / 147 |
| 2026-09-24 | 67a91a5 | §4.5.527 | 8427 | 34 | 27 / 0 / 27 | 140 / 80 / 60 | 7 / 0 / 7 | 24 / 19 / 5 | 103 / 88 / 15 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 16 / 15 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 372 / 219 / 153 |
| 2026-09-24 | 72c07cf | §4.5.528 | 8440 | 34 | 27 / 0 / 27 | 144 / 80 / 64 | 7 / 0 / 7 | 24 / 19 / 5 | 103 / 88 / 15 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 16 / 15 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 376 / 219 / 157 |
| 2026-09-25 | b7df5e5 | §4.5.529 | 8466 | 34 | 27 / 0 / 27 | 154 / 81 / 73 | 7 / 0 / 7 | 24 / 19 / 5 | 105 / 90 / 15 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 16 / 15 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 388 / 222 / 166 |
| 2026-09-25 | f929412 | §4.5.530 | 8485 | 34 | 27 / 0 / 27 | 155 / 79 / 76 | 7 / 0 / 7 | 24 / 19 / 5 | 105 / 90 / 15 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 16 / 15 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 389 / 220 / 169 |
| 2026-09-25 | b70759a | §4.5.531 | 8505 | 34 | 27 / 0 / 27 | 158 / 81 / 77 | 7 / 0 / 7 | 24 / 19 / 5 | 105 / 90 / 15 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 16 / 15 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 392 / 222 / 170 |
| 2026-09-25 | 8cb7eef | §4.5.532 | 8505 | 34 | 27 / 0 / 27 | 158 / 81 / 77 | 7 / 0 / 7 | 24 / 19 / 5 | 105 / 90 / 15 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 16 / 15 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 392 / 222 / 170 |
| 2026-09-25 | 581e26a | §4.5.533 | 8512 | 34 | 27 / 0 / 27 | 161 / 81 / 80 | 5 / 0 / 5 | 24 / 19 / 5 | 105 / 90 / 15 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 16 / 15 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 393 / 222 / 171 |
| 2026-09-25 | 5a54c76 | §4.5.534 | 8520 | 34 | 27 / 0 / 27 | 161 / 81 / 80 | 5 / 0 / 5 | 24 / 19 / 5 | 105 / 90 / 15 | 12 / 0 / 12 | 14 / 9 / 5 | 6 / 0 / 6 | 16 / 15 / 1 | 17 / 8 / 9 | 4 / 0 / 4 | 2 / 0 / 2 | 393 / 222 / 171 |

- 2026-09-21: one row for SIX slices (§4.5.519–524), an owner directive that took all six startable
  rows of the §2 start-order table sequentially with scoped gates and one adversarial review at the
  end. 2T drops to `0` startable: row 5 is deleted, 🆕 L ⓢ, 🆕 I ⓖ, 🆕 N's two spelling cells and
  🆕 O's eleven-reader class are closed inside their rows, and row 32 was re-measured and
  reclassified ORACLE-SPLIT. 2M and 3b rise because the batch RECORDED its residues (+13 mechanism
  bullets, +3 §3.b rows), which is the intended direction. `format_version` 32 → 33.
- 2026-09-21 (§4.5.525): 2M rises although SIX rows were deleted — the slice recorded eleven
  residues (four of them oracle splits, two one-oracle, one an unmeasured matrix column), so 2M is
  140 − 5 + 7 and its startable column drops twice: five deleted rows were startable, and the
  surviving modport row was re-measured as ORACLE-SPLIT. 3b rises by the four §3.b rows the same
  residues opened. `format_version` 33 unchanged. HEAD `c026a81`.
- 2026-09-24 (§4.5.526): 2M falls by five — nine bullets deleted (the four slice rows plus five
  neighbours re-measured as closed on the frozen POST2 binary: the `!trusted_w` carve-out, the
  frame-call mirror sign, the wide `$random` actual, a class field's extension sign in "Size cast",
  and the "Class fields" `ir_bits_of` row) against four residues recorded (two inline-lane rows,
  one "Real" row, one oracle split); two more residues went into existing rows (the enum-storage
  row, §3.b `x→real`), so they add no count. 3b rises by two §3.b rows. 6 is a RECOUNT plus one: the
  beside-track list held nine bullets where the Summary said eight, and the slice added the
  `builtins` lowering-rows bullet. `format_version` 33 → 34.
- 2026-09-24 (§4.5.527): 2T rises by one — 🆕 R (the shared wide walk inside self-determined
  positions and on the §11.8.2 sign, WALL) is the prerequisite the slice's D8 stop filed. 2M is
  137 − 3 + 6: deleted the "Index sealing" `~128'd0` bullet and the stale constant-domain C2 / C3
  (3-tool identical at HEAD); added five "Index sealing" residue bullets and one "Oracle splits"
  bullet. Startable 82 − 3 + 1 (only the declining-fold bullet has no prerequisite; three wait on
  🆕 R, the fill bullet on row 30, the oracle bullet is a disqualification). Residues that are new
  cells of existing rows (rows 15 and 16, the `parameter unsigned` bullet, the iverilog-hang bullet,
  §3.b `defparam-iface`) add no count. 3b falls by the stale `wide-override` row.
- 2026-09-24 (§4.5.528): 2M is 140 − 4 + 8. Deleted: the "Inline / frame binds" HIERARCHICAL-leaf
  bullet (closed) and the three stale "Real" bullets R1 / R2 / R3 (`fa(1) + (-s)`,
  `p::one() + (-s)` / `c.getr() + (-s)`, `r = (-s)` / `r = (s+s)`; 3-tool identical at HEAD).
  Added: five "Inline / frame binds" bullets (the inexact-fold prerequisite row, the real-returning
  hierarchical call, the multi-dimensional select, the declined declarations, the inline stream),
  one "Scoping" bullet (a function-local block label beside an instance) and two "Oracle splits"
  bullets (`u.w[u.P*2-1:0]`, `$bits(u.r)`). Startable 80 − 4 + 4 (the four deleted bullets were
  startable; the real call, the select, the declined declarations and the stream have two oracles
  or a hand-IEEE plan; the inexact-fold bullet waits on its prerequisite, the block-label bullet
  and the two oracle-split bullets are splits). Two residues are new cells of existing rows (the
  widening cast of a signed hierarchical call into the size-cast impure-operand row, `mg[u.hs(1)]`
  into the index-sealing function-call row) and add no count.
- 2026-09-25 (§4.5.529): 2M is 144 − 3 + 13. Deleted: the "Delays / events" D10, D11 and D12
  bullets (CL-09, taken by the slice). Added: five "Delays / events" bullets (the `$finish` drain
  prerequisite, the all-constant header list it blocks, index liveness behind a concatenation / system
  function / hierarchical name, the non-admitted bodies, the header waiter's same-step re-run) and
  eight "Oracle splits" bullets. Startable 80 − 3 + 4 (the three deleted bullets were startable; the
  drain, the index-liveness, the body and the re-run bullets have two oracles; the all-constant
  bullet waits on the drain and the eight splits are splits). 3b is 103 + 2, both startable
  (`level-select-event`, 2-oracle after time 0; `edge-event`, 2-oracle). The x/z override cell went
  into §2 row 15 and adds no count.
- 2026-09-25 (§4.5.530): 2M is 154 − 6 + 7. Deleted, all startable: "Size cast / signedness" the
  impure-operand widening bullet; "Inline / frame binds" the `expr_is_repeatable` decline bullet
  (PRE = both oracles on its cells), `int'($random*1.0)` draw count and the widened signed
  non-repeatable actual (stale: PRE = iverilog); "Performance" the per-bit `coerce_two_state` bullet
  and the bind-lane O(declared width) bullet. Added: "Size cast / signedness" the fabricated-width
  bullet (blocked, prerequisite) and the operator-over-call size leaf (startable); "Inline / frame
  binds" the x-bearing queue element bind (startable); "Index sealing" the `gp[0][$urandom]` double
  draw (startable); "Diagnostics / artifacts" operator call counts (startable) and the `wprog` label
  shift (blocked, observation only); "Performance" the three surviving `coerce_two_state` sites
  (startable: the R2 site is open). The out-of-range real bullet absorbed the saturation residue and
  became a prerequisite row (startable → blocked). Startable 81 − 6 + 5 − 1 = 79; blocked 73 + 2 + 1 = 76.
  The prim-cast context census was folded into its existing bullet and adds no count.
- 2026-09-25 (§4.5.531): 2M is 155 − 1 + 4. Deleted (startable): "Delays / events" the `$finish` drain
  bullet. Unblocked: the all-constant header list row (its prerequisite closed; blocked → startable).
  Added: the `#0` cont-assign delivery order (startable), the deferred-action `$finish` text capture
  (startable, verilator + hand-IEEE), the `$fatal` immediate arm (held on purpose) and one oracle-split
  bullet. Startable 79 − 1 + 1 + 2 = 81; blocked 76 − 1 + 1 + 1 = 77. `format_version` 34 unchanged.
- 2026-09-25 (§4.5.532): 2M is 158 − 1 + 1. Deleted (startable): "Delays / events" the all-constant
  header list bullet. Moved blocked → startable: the t0 false event of a cont-assign-only wire (2
  oracles now, re-measured by both lenses). Added: one oracle-split bullet (time-0 process order
  around `always @(K)`). Startable 81 − 1 + 1 = 81; blocked 77 − 1 + 1 = 77. Test count unchanged
  (40 pins converted in place). `format_version` 34 unchanged.
- 2026-09-25 (§4.5.533): 2M is 158 − 1 + 4. Deleted (startable): "Delays / events" the t0 false
  event of a cont-assign-only wire. Added: the time-0 edge on a definite settle (startable, 2
  oracles), the copy-net value gap (one oracle, blocked), the per-net dirt of an unpacked array
  (one oracle, blocked) and one oracle-split bullet (iverilog's operator- and literal-decided
  time-0 wake). §2-N's t0-event residues 5 → 3 (`1'bx`, `assign #1`, multi-driver and the per-bit
  reader left: three wake nothing now, the fourth is loud). Startable 81 − 1 + 1 = 81; blocked
  77 + 3 = 80; §2-N blocked 7 − 2 = 5; total 392 + 1 = 393. Tests 8505 → 8512. `format_version` 34
  unchanged.
- §4.5.534: §2 "Delays / events" the time-0 edge bullet (startable) deleted and the phantom
  intermediate value of a variable-reading driver settled before its initializer (startable, M)
  added; the `#0` cont-assign and the oracle-split bullets extended. Startable 81 − 1 + 1 = 81;
  blocked 80; total 393. Tests 8512 → 8520. `format_version` 34 unchanged.
