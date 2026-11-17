(§5.2 Q-HIERBITS · rank ① · source §2 bits-hier-placeholder's fix line, W/wrext/fix1/PLAN.txt (P), Q-MDRVMIX's park (2026-11-17)) (`$bits` of a hierarchical name lowers as a provisional constant, patched late):
* purpose: `$bits` of a hierarchical name lowers as the Signal placeholder with a leaf shape that a hierarchical parameter (`u.P`) already takes, so no lowering-time fold can read a provisional value (IEEE 1800-2017 §20.6.2)
* why: no goal; the root of §2 bits-hier-placeholder's silent wrong values (a case label, a replication, index sealing, and face 2's select stores and reads) and of the stale write view that §5.2 Q-WEXTPOS stands down on; trigger: a design of the set that spells `$bits` of a hierarchical name in an IR position
* needs: —
* readers: Q-WEXTPOS (its stand-down is deleted when this lands)
* state: OPEN (2026-11-17)
* record: as filed below ; superseded order lines are kept for the record
* the root (W/wrext/fix1/PLAN.txt (P)): `lower_bits_fold` (`expr_special.rs`) lowers `$bits(u.X)` to a provisional `Const` 32 kept in `undecided_placeholders`, which `resolve_deferred_hier_bits` (`hier_defer/bits.rs`) patches in place in the end phase; a reader that folds it before then (`norm_sub_k`, `norm_k_sub` in `packed.rs`, `const_bound` in `arrays/pattern.rs`, a case label, a replication count, index sealing) keeps the 32; lowered as `u.P` is (a Signal placeholder with `HierLeafShape { 32, signed }`; w10 right and quiet), no reader can fold it
* blast radius: every design with a hierarchical `$bits` in an IR position, many through interface members that take the local path (PLAN.txt (P)); a census of the placeholder's value readers comes first (the §2 entry's fix line)
* when it lands: face 2's cells are right (§2 bits-hier-placeholder), a design of the lens-sound a1 shape gets its decided extent (a4 becomes loud under Q-MDRVMIX's rule), and Q-WEXTPOS's flag has no producer, so its stand-down is deleted
* note: §5.2 75 (DH, the deferred hierarchical reference funnel) takes the `$bits(u.X)` placeholder only if its census shows the same producer (75.md); this row is that placeholder alone
* source: W/wrext/fix1/PLAN.txt parts 1 and 3, premise/REPORT.txt K5, K9; §2 bits-hier-placeholder
