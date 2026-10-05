# §4.5.591 r2 SOUNDNESS lens (delta) — REPORT (live)
round: 2
status: COMPLETE. lens verdict: PASS (0 BLOCKING, 0 MAJOR, 2 MINOR)
post_b $S/s591/post_b/vita md5 1d520f3399f0cd76ed243b6a24cd86cb (release); diff b63db492894e01018bdda4acb21d3991
cells: r2/sound/cells/c4*.sv (7 new) + r2/sound/cells/r1/ (31 r1 cells re-run); runner r2/sound/runb.sh (PRE, post_a, post_b, post_b staged, iverilog, sv2v)

## Q4 r1 cells on post_b (one-shot; staged == one-shot 31/31)
F1 (c22 c23 c24 c26 c27 c30 c33 c38 c39): PRE | post_a loud | post_b = PRE = iverilog = sv2v (e.g. c22 `rp A=3 P=3`, c33 `pwn A=..625 B=..625`). FIXED.
F2 c01: PRE `m13 small` b=4 | post_a same | post_b `m13 big` `b=10` = iverilog/sv2v/verilator. FIXED.
c07 `pr A=3 B=5`, c32 `pnw A=3 B=9`: post_a loud -> post_b = PRE (wrong; oracles B=3; pinned KNOWN-WRONG per IMPL).
c25 nn ref-between: PRE = post_a = post_b loud, oracles `rn A=3 P=3` (pre-existing residue).
c28: PRE `xw small` -> post_a/post_b `xw big` (both silent; iverilog/sv2v refuse). c29: post_b loud (ivl/s2v refuse).
c04/c05 PRE = post_b `sa P=5` / `ra P=5` (residue F4 unchanged). Others unchanged.

## Q1 segment census (scope_item_starts / module_item_lo, package.rs:383-456)
module_item_lo = exhaustive match over ast::ModuleItem (NetVar..Error, no catch-all); + header params, ANSI/non-ANSI
ports, bare-region GenItem::Item/For/If/Case/Block. Not ModuleItems: `specify` = E2002 parse error on PRE/post_b (c41;
iverilog `sp P=3`, sv2v parse error) -> unreachable. timeunit / bind / nested module / attribute-only: not run
(UNVERIFIED); a dropped item can only matter if it references the name.
Cells with a referencing item between two wildcard imports (nw pair), post_b = PRE = iverilog = sv2v, staged = one-shot:
c42 `include "mid.svh"` (localparam int A=P): `inc A=3 P=3` (post_a loud) — include text segments by buffer offset.
c43 macro `USE_P`: `mac A=3 P=3` (post_a loud). c45 function body reading P: `fn f=3 P=3` (post_a loud).
c47 bare generate-region `if (P > 1)`: `gi g` `gi P=3` (post_a loud).
c44 two files (padded `$unit` Q in a.sv, adjacent imports in b.sv): PRE `mf P=3 Q=1` | post_a = post_b = staged E3010
  top.P | iverilog `Ambiguous use of 'P'. It is exported by both 'pa' and by 'pb'.`, sv2v `ambiguously refers` -> no
  cross-file mis-segmentation.

## Q2 R7 (drop the decl.span retain) — EQUIVALENT, proved by census
Segments are used only in imports_adjacent (equality of two Some(seg)); every seg of one scope comes from ONE starts
vector (instance.rs:529, iface_inst.rs:219, package body). Items with lo < decl.lo precede every in-declaration import
(+k on every seg); items with lo >= decl.hi follow every import (counted in none). Equality is unchanged, so R7 is
equivalent whenever every segmented import lies inside decl.span: segmented imports come from decl.body / bare-region
items (cu imports are ImportSite::Unit = None; inject_cu_items copies only Typedef/Param/Func/Task, never an import).
Holds even for cross-buffer spans (stronger than IMPL's "copied items precede the declaration").

## Q3 F2 fix — insert/read order census (package.rs)
explicit_imports writers: 1693 (wide), 1711 (narrow), 1775 (var). readers: 1433/1501/1553 (wildcard skip), 1651 (pair
check), 1691-1692 (import_bound). Wide arm order: pair check 1651 -> import_bound 1691 -> insert 1693 (read before
insert: correct). wc_origin writers 1464/1483, 1522/1535, 1569/1589; readers wildcard_prior 367, import_bound 1691,
var arm wildcard_bound 1749.
Residue (write twin, same root class as F2 / `$unit`+module one import scope): the explicit VARIABLE arm clears
wide_param_bits only `if wildcard_bound` (1749), not over a prior `$unit` explicit wide. c40: `$unit` `import pa::W;`
(wide) + module `import pb::W;` (var): PRE = post_a = post_b = staged `uv W=18446744073709551625 b=8`;
iverilog `uv W=5 b=8`, sv2v `uv W=5 b=8`.

verilator (raw): c40 `uv W=5 b=8`, c42 `inc A=3 P=3`, c43 `mac A=3 P=3`, c45 `fn f=3 P=3`, c47 `gi g` `gi P=3`;
c44 not measured (harness quoting: `Cannot find file containing module: 'c44a.sv c44b.sv'`).
Rewritten claims re-read: test header lines 6-8 ("a `localparam int` ask the >64-bit map first ... `xw A=9`") and the
package.rs explicit-wide comment (cites m13 / n01) agree with the measurements (F3, F2 closed).

## Q5 mutants (copy of post_b tree, CARGO_TARGET_DIR=$S/s591/target_rv_sound2, debug, deleted after)
| id | mutation | scoped (2 test files) | --workspace | cell |
| N1 | scope_item_starts: bare-region GenItem::For/If/Case/Block push nothing | SURVIVED 36/36 | SURVIVED 9117/9117 (15 skipped) | c47: N1 E3010 `generate-if condition is not a constant: undefined name P` + E3010 top.P; post_b = PRE = 3 oracles `gi g` `gi P=3` -> NOT equivalent: test gap |
| R7 | (implementer's) drop decl.span retain | - | - | equivalent, proved by census (Q2) |

## Findings round 2
| sev | finding | class |
| MINOR | N1 survives --workspace although it turns c47 (bare generate-region `if (P > 1)` between two wildcard imports) correct->loud: no test pins a generate construct as the separating item | test gap, same root class as F1 |
| MINOR | c40 explicit VARIABLE over a `$unit` explicit wide keeps the wide binding (PRE = post_b) | same root class as F2 ($unit + module one import scope), pre-existing |
r1 F1 BLOCKING: closed (9/9 cells = PRE = 3 oracles, staged = one-shot). r1 F2/F3/F5: closed. r1 F4: unchanged residue.
