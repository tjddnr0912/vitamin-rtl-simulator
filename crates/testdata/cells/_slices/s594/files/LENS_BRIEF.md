# §4.5.594 review brief (both lenses)

Slice: §2 🆕 AI — a signed constant beside a parameter-count replication `{N{…}}` or a constant-array element `A[i]` was sign-extended inside an unsigned region because `const_self_width` could not size those leaves. Spec $S/s594/PLAN.md; build record $S/s594/IMPL.md; background $S/s594/GROUNDING.md.
Diff: $S/s594/post_a/wt.diff (md5 41d06f63…; worktree $S/s594/wt based on main 1da347bc — READ ONLY).
PRE: $S/s594/pre/vita md5 fc478eb9778be05e6d4ef7afa884443e (release, 1da347bc; staged pre/sep). POST: $S/s594/post_a/vita md5 4b3afc99cfeb6a2bc5bb94cfdeca3596 (release; staged post_a/sep). Frozen. Mutant/probe builds only in your own copy with CARGO_TARGET_DIR=$S/s594/target_rv_<lens> (CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0; ≥15 GiB free; delete by literal path at the end).

Claim under review:
- Replication count sized by `const_placement_count` (`fold_count` through the shared placement resolver); a whole element `A[i]` sized and signed by `const_elem_wsign` (from `const_array_elem_read`), in `const_self_width`, `const_signed_env` and `const_expr_signed`.
- Region rule (`enum LeafRule {Literal, Scope}`, const_leaf_rule.rs): a region root that contains a user `Call` or names a constant-function variable (`envw`) keeps PRE's leaf rules for width AND sign; a comparison's two operands share one rule; package function bodies keep PRE's rules (D1); a tier-3 bound checks every node under its root's rule (D2); a zero name count behaves like its literal twin (D3).
- Measured on 3082 cells: WRONG→OK 298, LOUD→OK 20, no-two-oracle-agree 31 (land on verilator / iverilog+sv2v for delays), WRONG→WRONG′ 8 (untyped localparam value lane: width now right, value from the unlimited fold — residue V, ROADMAP :199/:200), OK→anything 0, LOUD→WRONG 0; staged = one-shot 3158/3158; 666 harness 0 movers; corpus .vu/.velab 30/30 byte-identical (count arm fires 8× Scope + 8× Literal in verilog-axi; element arm never fires on the corpus); elab time unchanged. Mutants M1–M11 killed, M12 equivalent.
- Residues: R1 call count, R2, R3 `S.len()`, R4 65-bit signed neighbour, R5 region-rule cells kept at PRE (🆕 AE), Pkf_cntE (package function with a name count, `L=0`, oracles `L=1`).

HIT OUTSIDE THE TABLE. Every lens answers: does the product shake (a new silent-wrong; a correct→loud; a staged/one-shot divergence; a crash)? For each finding: new vs known class; PRE | POST | iverilog | verilator | sv2v raw; 4-way; attribution re-measured on PRE.
Severity: BLOCKING / MAJOR (false claim in comments/docs/commit) / MINOR (residue).
Budget: ≤60 tool calls, ≤40 designed cells. Return findings in your final message (try $S/s594/r1/<lens>/REPORT.md first).
