(history 2026-11-21 · from ROADMAP.md, goals/G-EH1.md, 5.2 docs and IEEE_CHECKLIST.md as drafted 2026-11-20) (the G-EH1 chain before Q-LOSSYSPELL's park):
* moved by batch 6's landing docs commit (§4.5.655; owner, 2026-11-21, "1안 진행", W/batch6/RULINGS.txt L2-69, L2-73): Q-LOSSYSPELL is parked behind the new structural row §5.2 Q-FILLCTX and batch 6 is Q-WEXTPOS alone, so the State line, the O5 cut, the Current chain, G-EH1's chain and lines, §5.2's table line and README State, and the lines of 24.md, 29.md, Q-MDRVMIX.md, Q-SCOPEBIND.md, Q-LOSSYSPELL.md and mdrv-park-2026-11-20.md that said Q-LOSSYSPELL was built or landing say so; the checklist lines re-read for the round's filings change too
* what stands below: each line the landing docs draft of 2026-11-20 (round 2, never committed; saved as W/batch6/landing/r3/draft_round2.patch and draft_round2.untracked/) held and this commit replaces, verbatim, by file; how many of a block's lines also stood at bcd5afec is said on the block's head bullet (the checklist's ten leaf and table lines; every other line is the draft's own)
* the replacing lines are in the live files; the G-EH1 table rows 3–9 of the draft stand renumbered there, unchanged but for their number and, for row 24, its why
* from docs/ROADMAP.md:

```text
2026-11-20: goal G-EH1 is current. Batch 6 (Q-WEXTPOS, Q-LOSSYSPELL) is built and at its boundary verify; Q-MDRVMIX, built again in batch 6, is parked
(owner, 2026-11-20) behind the new structural row Q-SCOPEBIND and left the batch (§4.5.655). The chain's next row is Q-SCOPEBIND (batch 7).
G-EH1's cut by O5 on its chain of 2026-11-20 (9 rows; re-read at Q-SCOPEBIND's plan, whose cut goes to the owner): batch 6 = Q-WEXTPOS, Q-LOSSYSPELL;
batch 7 = Q-SCOPEBIND, Q-MDRVMIX (a boundary before 24, which parked before, two rows stacked; Q-SCOPEBIND alone if it bumps a format or schema hash);
batch 8 = 24 (EH1 cut), Q-OVRXPLANE, Q-LITLANE (a boundary before 29's page half); batch 9 = 29 (page half), 31 (the goal's last row).
G-EH1, after batch 5 (landed, §4.5.654); batch 6 = rows 1–2, batch 7 = rows 3–4, batch 8 = rows 5–7, batch 9 = rows 8–9 (O5, re-read at Q-SCOPEBIND's plan):
| 2 | Q-LOSSYSPELL | which lossy constant producers a design spells, recorded once per design | BUILT |
| 3 | Q-SCOPEBIND | a name is bound to a declaration the source's scopes do not give it | OPEN |
| 4 | Q-MDRVMIX | overlapping partial and mixed writes of a packed variable are not refused | PARKED Q-SCOPEBIND |
| 5 | 24 (EH1 cut) | struct member part-selects with named bounds, as reads and write targets | PARKED Q-LOSSYSPELL, Q-SCOPEBIND, Q-MDRVMIX |
| 6 | Q-OVRXPLANE (typed lane) | an override's sized x / z literal loses its unknown plane | OPEN |
| 7 | Q-LITLANE | `const_eval_u32` reads a sized signed literal as its unsigned pattern | OPEN |
| 8 | 29 (page half) | element selects nested in a write target are refused | PARKED Q-LOSSYSPELL, Q-OVRXPLANE, Q-LITLANE |
| 9 | 31 | VeeR EH1 corpus row: the hello_world test passes on unmodified RTL | WAITS 24, 29 |
| Q-LOSSYSPELL | R8.1 · L1 (built) | [which lossy constant producers a design spells, recorded once per design](roadmap/5.2/Q-LOSSYSPELL.md) | — F |
```

* from docs/roadmap/goals/G-EH1.md:

```text
* before it: batch 5 landed on main on 2026-11-18 (§4.5.654: PARSE-1's provenance piece and Q-WREXT, each structure only); Q-MDRVMIX, built in batch 5, was parked on 2026-11-17 (owner, option A) and its commits pulled out (a33df0e0); built again in batch 6 over Q-WEXTPOS, it was parked again on 2026-11-20 (owner, Q3 option 3) and its commits pulled out (17703a0c), so it is on this chain behind the structural row Q-SCOPEBIND.
* chain (run order; batch 6 = rows 1–2, batch 7 = rows 3–4, batch 8 = rows 5–7, batch 9 = rows 8–9, Order rules O5 applied to the 9-row chain of 2026-11-20, re-read at Q-SCOPEBIND's plan; the owner's Q1 (a) cut was made for the 6-row chain):
| 2 | Q-LOSSYSPELL | record | BUILT | the design-level term (which lossy constant producers a design spells) has three readers, row 24, 29's page half and R6.1 (Q-VALRULES), and no record holds it (O2 (a)); owner Q2 (a) of the roadmap review, 2026-11-17; built in batch 6 |
| 3 | Q-SCOPEBIND | record | OPEN | Q-MDRVMIX's second park (2026-11-20): a name is bound to a declaration the source's scopes do not give it, through five doors; the owner ordered it fixed at its root now, before Q-MDRVMIX re-enters and before row 24 (Q-SCOPEBIND.md) |
| 4 | Q-MDRVMIX | prerequisite | PARKED Q-SCOPEBIND | row 24 waits on it (owner option 2, 2026-11-16); built in batch 5 and in batch 6 and pulled out both times, it returns by a fresh plan once Q-SCOPEBIND lands (Q-MDRVMIX.md, the parks and re-entry) |
| 5 | 24 (EH1 cut) | design | PARKED Q-LOSSYSPELL, Q-SCOPEBIND, Q-MDRVMIX | opened narrowed to the tokens EH1's original source uses (owner, 2026-11-16); it waits on Q-MDRVMIX, parked again 2026-11-20 behind Q-SCOPEBIND, and it re-enters by a full re-plan that reads the term (24.md) |
| 6 | Q-OVRXPLANE (typed lane) | prerequisite | OPEN | 29's page half waits on it (29.md); its typed-target lane reads Q-DECLTYPE's S12 record (landed, §4.5.635; its list line left with the landing) and row 48's D4 (delivered), while its untyped-keyword targets stay behind S1v-V, DORMANT in K2 (Q-OVRXPLANE.md) |
| 7 | Q-LITLANE | prerequisite | OPEN | `const_eval_u32`'s literal lane is the producer of 29's r1, and the page half waits on it (29.md, Q-LITLANE.md) |
| 8 | 29 (page half) | design | PARKED Q-LOSSYSPELL, Q-OVRXPLANE, Q-LITLANE | holds EH1's lines on element selects nested in a write target; reads the term in place of S1v-V (owner O2, 2026-11-15); its full form stays PARKED behind Q-IDXSIGNCTX and Q-IDXVALUE, off this chain (29.md) |
| 9 | 31 | corpus | WAITS 24, 29 | the reach test: the first testbench run on unmodified EH1 RTL at HEAD; row 26's testbench site stays precomputed strings in the first-party testbench (31.md) |
* records: Q-WEXTPOS, row 1 (the write view's stand-down and each write's position; reader Q-MDRVMIX); Q-LOSSYSPELL, row 2 (the term; readers 24, 29's page half, R6.1); Q-SCOPEBIND, row 3 (every name bound by the source's scopes; readers Q-MDRVMIX, 24); Q-WREXT's write-extent view (landed, batch 5, §4.5.654); BIND-1's home (landed, §4.5.640) under Q-OVRXPLANE; LitOrigin, PARSE-1's provenance piece (landed, batch 5, §4.5.654), the fact Q-LOSSYSPELL can be computed over.
* note (2026-11-20, §5.2 Q-LOSSYSPELL's grounding F8, a correction of the line above, kept as filed): the record is not computed over LitOrigin, which records only which code built an `IntLit` ("Nothing else is claimed", `hdl-ast` `LitOrigin`): the signing keyword lives in `ParamDecl.dtype.signing`, a typed `let` formal is in no AST field, the Q-AESINK mark is a run fact (`const_seed_reads`) and a `-G` value has no source literal; Q-LOSSYSPELL as built is a token scan in the parser with the parser's marks, ORed with the `-G` values at `Elaborator::run`'s head (Q-LOSSYSPELL.md)
* size: ESTIMATE, 2026-11-17, made for the 6-row chain: 6 listed rows → 8 to 10 run, 2 batches (cut before 29, O5), 2 to 3 weeks; the 8-row chain after Q-MDRVMIX's park is 3 batches (O5) and is not re-estimated; the 9-row chain after its second park is 4 batches (O5), and Q-SCOPEBIND's size is unknown until its grounding and plan (not re-estimated). What changes it: the two signals above and Q-SCOPEBIND's cut.
* batches: no chain row has landed yet; batch 5 (PARSE-1's provenance piece, Q-WREXT), the work before row 1, landed 2026-11-18 (§4.5.654); batch 6 = rows 1–2 (Q-WEXTPOS, Q-LOSSYSPELL), built and at its boundary; batch 7 = rows 3–4, batch 8 = rows 5–7 and batch 9 = rows 8–9, planned (O5, re-read at Q-SCOPEBIND's plan).
```

* from docs/roadmap/5.2/README.md:

```text
* State (2026-11-20): batch 6 built on the integration branch, at its boundary (§4.5.655): Q-WEXTPOS (adf7656e, f4a504ba, e37c4a08, fix F1 67a8603c, 03838d6d, f113a21e, 3da4d685) and Q-LOSSYSPELL (01223718, 12e978e9, 4e6bf466, fix N4 8f0b86b8), each structure only but for named moves; Q-MDRVMIX, built again (303088c7, 779ecae1, 6bc9babc), parked at R8.1 · L1 behind the new structural row Q-SCOPEBIND (owner, 2026-11-20, Q3 option 3) and pulled out (17703a0c); next Q-SCOPEBIND (batch 7), then Q-MDRVMIX, then 24
```

* from docs/roadmap/5.2/Q-LOSSYSPELL.md:

```text
* needs: nothing landed is missing (as filed below)
* state: BUILT (2026-11-19)
```

* from docs/roadmap/5.2/24.md:

```text
* slot (2026-11-20, Q-MDRVMIX's second park): PARKED behind §5.2 Q-MDRVMIX and §5.2 Q-SCOPEBIND (owner, 2026-11-20, Q3 option 3): Q-MDRVMIX was built again in batch 6 over §5.2 Q-WEXTPOS (303088c7, 779ecae1, 6bc9babc) and pulled out of it (17703a0c), parked behind the structural row Q-SCOPEBIND, which binds every name by the source's scopes; this row re-enters after both, by the full re-plan line 8 names; §5.2 Q-LOSSYSPELL, the record :5 names, is built in batch 6 (U1 01223718, U2 12e978e9, U3 4e6bf466, fix N4 8f0b86b8) and lands with it; the K7 lines above stay as they read on 2026-11-17 (no rule of Q-MDRVMIX is in `crates/`)
```

* from docs/roadmap/5.2/29.md:

```text
* slot (2026-11-20, batch 6): Q-WEXTPOS and Q-LOSSYSPELL are built on the batch 6 integration branch (§4.5.655), not yet on main; Q-MDRVMIX, built again there, is parked behind §5.2 Q-SCOPEBIND (owner, 2026-11-20), so the order now is Q-WEXTPOS → Q-LOSSYSPELL → Q-SCOPEBIND → Q-MDRVMIX → row 24 (EH1 cut) → Q-OVRXPLANE (R7.4) → Q-LITLANE (R7.5) → this row's page half → row 31 (G-EH1's chain, ROADMAP.md); the page half's wait on 24.md:5's term is a wait on §5.2 Q-LOSSYSPELL's record (`sign_kw_param`, the signing keyword on an untyped parameter, read at `Elaborator::run`'s head by its READER PROTOCOL), and its waits on Q-OVRXPLANE (R7.4) and Q-LITLANE (R7.5) stand
```

* from docs/roadmap/5.2/Q-MDRVMIX.md:

```text
* slot: parked again 2026-11-20 at R8.1 · L1 behind §5.2 Q-SCOPEBIND (owner, 2026-11-20, Q3, option 3): built a second time on the batch 6 integration branch over §5.2 Q-WEXTPOS, its three commits are pulled out of batch 6 by 17703a0c, and batch 6 goes on to its boundary as Q-WEXTPOS and Q-LOSSYSPELL; nothing of this row is on main or on the batch 6 branch; row 24 waits on it as before
* built and pulled (2026-11-20): on the batch 6 integration branch over Q-WEXTPOS, 303088c7 (rule X), 779ecae1 (rule Y) and 6bc9babc (the widening to `always_ff` and `always_latch` pairs, and a whole `always_latch` write beside another writer promoted, the owner's decisions of 2026-11-18 below), reverted together by 17703a0c, so `crates/` is again Q-WEXTPOS's tree with Q-LOSSYSPELL's commits on it; the code as built is W/mdrv2/build/c1.patch, c2.patch and c3.patch, its pin inputs W/mdrv2/build/pins and the expectations W/mdrv2/build/expected_c1.tsv, expected_c2.tsv and expected_c3.tsv; the fourth unit C4, the shadow stand-down, was never committed: its tree state is saved under W/mdrv2/park/c4 (tracked.diff, untracked.tar, MD5S.txt, status.txt) with W/mdrv2/build/c4.patch, and it is the re-entry's input, not a design to restore (the stand-down exists only because the binding is wrong: W/mdrv2/sdelta/RULINGS.txt C28)
```

* from docs/roadmap/5.2/Q-SCOPEBIND.md:

```text
* slot: R8.1 · L1, the next row after batch 6 (Q-WEXTPOS, Q-LOSSYSPELL) lands, before §5.2 Q-MDRVMIX and row 24 (owner, 2026-11-20, Q3 option 3; W/mdrv2/RULINGS-owner.txt, W/mdrv2/sdelta/RULINGS.txt C28)
```

* from docs/roadmap/history/mdrv-park-2026-11-20.md:

```text
* moved by batch 6's landing docs commit (§4.5.655; owner, 2026-11-20, Q3 option 3): Q-MDRVMIX is parked again behind the new structural row §5.2 Q-SCOPEBIND and batch 6 is Q-WEXTPOS and Q-LOSSYSPELL, so the State line, the O5 cut, the Current chain, G-EH1's chain and §5.2's two table lines say so; what they said before stands here verbatim
```

* from docs/IEEE_CHECKLIST.md; 10 of these 15 lines also stood at bcd5afec (the nine leaf lines and the section 10 table line):

```text
10.3, 10.4, 23.3.3.2 and 26.3 lines moved no box. Batch 6's docs (§4.5.655, 2026-11-20) re-read 3.12.1, 5.7.1, 6.5, 6.16,
6.19, 6.20.6, 6.23, 8.25, 9.2.2.2–9.2.2.4, 9.3.1, 9.4.5, 10.3.2, 10.6.1, 10.6.2, 11.12, 13.3, 20.7, 23.3.2, 23.9 and 29:
eight boxes emptied on the filings' cells (9.4.5, 11.12, 29.3.1, 29.7, 29.8, 6.19.1, 8.25, 13.3), none checked.
Q-MDRVMIX is parked again (2026-11-20, behind Q-SCOPEBIND): an item only it completes stays empty.
| 10 | Assignment statements | 8 / 25 |
| total | | 266 / 898 |
- [ ] 7.5 Dynamic array element read and write — out-of-range 2-state reads x; §2 dyn-oob-reads-x, §2 queue-dyn-index-unsealed
- [ ] 8.5 Object properties — integral only; §3.b class-real-members, §2 class-field-default-width
- [ ] 9.4.2.3 Conditional event controls (`iff`) — §5.2 ENG-4
- [x] 10.4.2 Nonblocking procedural assignments
- [x] 10.9.1 Array patterns: positional and `default:` on fixed-size arrays
- [ ] 11.3.1 Operators with real operands — §2 signed-of-real-accepted, §0 8ⓔ
- [ ] 11.4.12 Concatenation operators — unsized operand accepted; §2 unsized-concat-operand
- [ ] 11.5.1 Select partly or wholly out of range — refused or wrong; §5.2 Q-PSELOOR, §5.2 Q-VMSELSHIFT, §2 packed-element-overhang
- [ ] 12.7.1 Comma-separated for-init or step — refused at parse; not filed
```
* the other lines of bcd5afec this landing replaces, written by the round-2 draft and not kept elsewhere (the checklist's re-read and recount, the manual's `$size` row, the head blocks' state and needs lines of Q-WEXTPOS, Q-MDRVMIX and Q-LOSSYSPELL), verbatim, by file:
* from docs/IEEE_CHECKLIST.md at bcd5afec:

```text
10.3, 10.4, 23.3.3.2 and 26.3 lines moved no box. Q-MDRVMIX is parked (2026-11-17): an item only it completes stays empty.
| 6 | Data types | 33 / 116 |
| 8 | Classes | 9 / 30 |
| 9 | Processes | 20 / 49 |
| 11 | Operators and expressions | 23 / 57 |
| 13 | Tasks and functions (subroutines) | 5 / 20 |
| 29 | User-defined primitives | 13 / 14 |
| total | | 274 / 898 |
- [ ] 3.12.1 Compilation-unit routines — fold at the calling module; §2 🆕 AF, §5.2 4
- [ ] 5.7.1 Unbased unsized `'0 '1 'x 'z` — widths in some positions; §2 🆕 J, §2 🆕 M
- [ ] 6.16 String variables: declare, assign, compare — interior NUL kept; §5.2 Q-STRNUL
- [x] 6.19.1 Enum with an explicit 4-state base
- [ ] 6.20.6 Const constants — status not recorded
- [ ] 6.23 Type operator `type(…)` — status not recorded
- [x] 8.25 Parameterized classes: value parameters
- [ ] 9.3.1 Block-local declarations — share flattened nets; §2 shared-flat-local-net, §2 blk-scope-name-leak
- [x] 9.4.5 Intra-assignment event control (`= @(ev)`, `<= @(ev)`)
- [ ] 10.3.2 Continuous assignment to a variable — partly; §3.b cont-var-partial, §3.b cont-var-gate
- [ ] 10.6.1 `assign` and `deassign` — deassign over a continuous assign accepted; §5.2 Q-DEASSIGNCA
- [x] 11.12 Let construct with untyped formals and an integral body
- [x] 13.3 Task declaration and enable
- [ ] 20.7 Array query functions — `$size` with a dimension argument; §2 size-dim-arg, §3.b dyn-size-spellings
- [ ] 23.9 Scope rules — names bound in the wrong scope; §2 declaration-order-accept, §2 struct-binding-by-name
- [x] 29.3.1 UDP header
- [x] 29.7 Sequential UDP initialization
- [x] 29.8 UDP instances
```

* from docs/manual/003_language-reference.md at bcd5afec:

```text
| `$size` `$left` `$right` `$low` `$high` `$increment` | Supported | `$low` is `min(left,right)`, `$high` is `max`, `$size` is `abs(left−right)+1`. |
```

* from docs/roadmap/5.2/Q-LOSSYSPELL.md at bcd5afec:

```text
* status: OPEN (no plan yet)
```

* from docs/roadmap/5.2/Q-MDRVMIX.md at bcd5afec:

```text
* needs: Q-WREXT, Q-WEXTPOS
* state: PARKED Q-WEXTPOS (2026-11-17)
```

* from docs/roadmap/5.2/Q-WEXTPOS.md at bcd5afec:

```text
* state: OPEN (2026-11-17)
```
* the two checklist leaf lines of bcd5afec this landing gives a reason line while their boxes stay checked (main's decision, W/batch6/RULINGS.txt L2-78 (d), (e)), verbatim:
* from docs/IEEE_CHECKLIST.md at bcd5afec:

```text
- [x] 23.3.2.2 Connecting ports by name
- [x] 28.3.3 Delay specification
```
