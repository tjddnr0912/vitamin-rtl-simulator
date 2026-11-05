(§5.2 Q-VALRULES · rank — F · source W/review/structure-2026-11-04/REPORT.txt S7 item 3) (census and design of the value and width rules spelled several times, no code):
* slot: just before row 74 (owner, 2026-11-05)
* slot: 74 (the expression-type funnel) takes its design
* scope: measure only, no code: a census of the value / width rules spelled several times, and a design
* evidence: 11,337 lines of constant evaluation in 13 `const_*` files
* evidence: `env_fold` "can land on a value the binder does not bind" (`elaborate/src/expr_size_hier.rs`)
* evidence: AST width walks against the lowering, policed by `VITA_SCW_CHECK` (`elaborate/src/expr_size_ctx.rs:1104`)
* evidence: the `LvalChunk` width three times (`elaborate/src/edge_gate.rs:1245` None against the whole-net fallback of `sim-engine/src/native/write.rs:393` and `sim-engine/src/state/init_diag.rs:1014`; the v36 history)
* evidence: 47 "N spellings" commits, 63da7bb8 the fifth walker-models-the-lowering
* fix: (a) `sim_ir::selfwidth::lval_chunk_width(ctx, chunk) -> Option<u32>` next to the canonical self-width rule, three callers, the undecided case loud
* fix: (b) one constant evaluator returning a typed value {bits, width, signed, domain} behind one entry, i64 only as a fast path checked equal on its domain
* fix: (c) width questions about an expression answered from its lowering (lower, then selfwidth), not by an AST walk that models it
* note: size (grep counts, not effort): (a) 1 new fn and 3 call sites in 3 files
* note: (b) and (c) 135 call sites (`const_eval_in_scope` 98, `const_eval_u32` 10, `const_self_width` 27), 102 `pub(crate)` Option fns in `const_*.rs` (ratchet b.) and 4 `expr_size` / `const_*width` files
