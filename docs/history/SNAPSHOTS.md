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
