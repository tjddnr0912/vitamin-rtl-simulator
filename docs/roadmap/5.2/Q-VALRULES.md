(§5.2 Q-VALRULES · rank — F · source W/review/structure-2026-11-04/REPORT.txt S7 item 3) (census and design of the value and width rules spelled several times, no code):
* slot: fix (d) runs first, as a code row at C0 · L1, before Q-SYMMIX's re-entry, S1v and batch 1 (owner, 2026-11-06)
* slot: just before row 74 (owner, 2026-11-05)
* slot: 74 (the expression-type funnel) takes its design
* scope: measure only, no code: a census of the value / width rules spelled several times, and a design
* note: fix (d) alone is a code row (owner, 2026-11-06)
* the census and the design (a)–(c) stay measure only
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
* evidence: a hierarchical signed leaf read in a size cast and in a plain assignment gives two values in one run (W/qdecltype/plan/cells/p18b_envfold_untyped: `r1=65535 r2=-1`, iverilog and verilator -1 -1 ; §2 hier-leaf-inline-region)
* evidence: the range count |msb − lsb| + 1 over self-determined bounds (IEEE 1800-2017 §7.4.1, §6.9.1) lives in elaborate (`elaborate/src/array_geom.rs:531` `range_to_dims_opt`, each bound by `const_range_bound_fold`) and three more times in the parser (`hdl-parser/src/sym_struct.rs:116` `sym_member_width`, `hdl-parser/src/packed_md.rs:122` `packed_dim_width` with `:105` `by_dir`, `hdl-parser/src/type_params.rs:628` `sym_range_width` with a `[X-1:0] -> X` shortcut), each wrong on mixed-sign bounds
* Q-SYMMIX parked behind (d) after three spellings failed (W/qsymmix/premise1, premise2, premise3)
* evidence: elaborate's constant `$bits(e)` (`elaborate/src/const_fn.rs:452`: `bits_of_view` | `bits_of_selfdet` | `wide_selfdet_width`) has no width for an unbased unsized literal under an operator (`elaborate/src/const_wide.rs` `fold_region`, `if ctx == 0 { return None; }`): op1_uu `$bits(N - '1)` -> `parameter B value is not a constant: $bits(…) has no constant-fold arm`, iverilog, sv2v → iverilog and verilator `A bn=5 bm=5 B=32` (W/qsymmix/premise2/cells2/tw_uu_hand)
* evidence: nor for a name whose declared range is ascending or not 0-based under an operator (`bits_of_selfdet`, `elaborate/src/expr_special.rs:730`, has no Binary arm, and `wide_selfdet_width`, `elaborate/src/const_wide.rs:1571`, reads the name through `narrow_param_bits`, `:1243`): h1e01_asc_lp `localparam logic [0:2] LA = 3'd5;` with the member width hand-spelled `|64'(($bits(LA + N))'(LA + N))| + 1` in a plain net's range PRE `t.sv:6:10: error[VITA-E3009] E-ELAB-UNSUPPORTED: a cast that is not a constant here is not allowed in a constant range bound [in top]` (W/qsymmix/premise3/h1new)
* evidence: the struct it spells, e01_asc_lp (`logic [LA + N : 0] a;`), is `A bv=14 ba=10` on PRE, iverilog, sv2v → iverilog and verilator
* the name alone (e16_hdr_asc_alone, `[PB : 0]` over `parameter bit [0:3] PB = 4'd3`) is `A bv=8 ba=4` on PRE, F1z and the three tools, so (d)'s fold must read each bound with its declared type (Q-SYMMIX premise judge r3 F2, W/qsymmix/premise3/cells2, cells4)
* evidence: a typedef alias of a symbolic struct (`typedef s_t a_t;`) copies the base's `Td`, `Packed`, `Enm` and `Union` slots but no `Sym` slot (`hdl-parser/src/typedefs.rs:617-631`), so the parser's `sym_layout` guards (`$bits` at `hdl-parser/src/type_params.rs:742`, the type-parameter value at `:553`, the cast at `:851`) do not fire for the alias
* its width reaches `sym_range_width`'s `[X-1:0] -> X` shortcut (`:635-645`): one more route a parser width spelling must cover, where (d)'s one node covers it (Q-SYMMIX premise judge r3 F1 and F3, W/qsymmix/premise3/cells, cells3)
* fix: (d) the parser's range count becomes one elaborate-folded node retiring the three parser spellings: either `$__vita_range_width(msb, lsb)` through the reserved `$__vita_` channel (no format change; an arm in each of the 11 elaborate evaluators that know `$clog2` and in `sim-ir/src/names.rs:96`) or a new AST node (format change; 34 files walk `ExprKind::Ternary`)
