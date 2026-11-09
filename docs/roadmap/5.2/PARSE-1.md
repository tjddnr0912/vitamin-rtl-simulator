(§5.2 PARSE-1 · rank — F · source W/program/reorder2-2026-11-09/PLAN.txt G1 PARSE, OPS) (the parser's constant fold in one file, its 64-bit kernels in hdl-ast):
* slot: R2.2 · L1, after SYS-1 (owner, 2026-11-09), with OPS, the operator and literal kernels
* slot: hdl-parser `params.rs` is in batch 1's diff, so it builds after batch 1 lands (D12)
* scope: structure only: every output equals PRE (values, diagnostics, exit code, `.vu` and `.velab` bytes, termination, cost), PRE's differences kept as named lanes (owner, 2026-11-09)
* scope: PARSE, the parse-time fold: a constant the parser needs to decide an AST shape (packed dims, struct layout, enum labels, type params, casts), over parse-time-known constants only, with no override, instance or binder yet
* note: its domain: literals and the parser's `const_locals` / `pkg_const_scoped` `ConstVal {v, w, signed}`, `w == 0` meaning width unknown (the width-aware twin declines); results `Option<i64>`, `Option<ConstVal>`, `Option<i128>`
* evidence: members `try_const_index` (`expr_primary.rs:33`), `try_const_index_w` / `_wc` + `cw_eval` (`:563` `:582` `:659`), `const_bound` (`:752`), `const_lit` (`:758`), `const_lit_enum`, `plain_int` / `plain_dec` (`packed_md.rs:563` `:640`), `lit_u32` (`packed_md.rs:42`), `decimal_literal` / `literal_element` (`structs/member_array.rs`), `written_decimal` (`structs/mod.rs:325`), `based_lit_pattern`, `const_param_fits`: 79 sites in 13 files
* scope: OPS, the kernels over operand values (IEEE 1800-2017 §11.4, §20.8.1 `$clog2`, §5.7): `const_binop` (`const_fn.rs:118`, 6 calls from 4 elaborate cores), `const_shl_i64`, `const_pow`, the parser's inline op arms (`try_const_index` {+,-,*,/}, `cw_eval` {+,-,*,/,%} i128 wrap, `const_lit` {+,-,*})
* evidence: the i64 `$clog2` formula is spelled at `const_fn.rs:676`, `expr_size_hier.rs:756`, hdl-parser `expr_primary.rs:60` and `:730`, and sim-ir `selfwidth.rs:165`; literal text → value at `parse_int_literal` (`literal.rs:331`, 18 callers) and 9 more readers
* fix: `hdl-ast/src/value_rules.rs`, beside `range_rules.rs` (hdl-parser and elaborate both reach it): `const_binop`, `const_shl_i64`, `const_pow`, `clog2_i64`, `wrap_i128` and the decimal-literal rule; elaborate's 6 `const_binop` calls and 2 i64 `$clog2` spellings call it
* fix: `hdl-parser/src/const_fold.rs`: the 15 parser fns verbatim, 79 sites in 13 files re-pointed by path ("an evaluator stays where its state is", `range_rules.rs` K4-1), its op lanes calling OPS
* note: arithmetic in two homes by crate reach (owner decision O2, 2026-11-09): 64-bit arithmetic and literal reading in hdl-ast (parser and elaborate), wide arithmetic in `sim-ir` `mw.rs` (elaborate and the engine; 13 kernels, with `const_wide_num.rs`'s 17 wide arms; the landed precedent)
* note: the other option, one new small crate below both (one `$clog2` spelling everywhere, today 7 in 4 crates), waits for a row that needs the parser to compute wide or x values; no parked row does (O2 (b))
* note: one home with VAL is not right: another phase (the parser must decline where elaborate binds) and crate (no edge to elaborate), so it shares OPS only
* note: lanes: each consumer's PRE op set (Parser4, ParserCtx5 i128, Lit3, Full) and each `$clog2` spelling's n ≤ 0 handling where they differ; every `?` chain verbatim
* pins: a token test (no `checked_*` over two folded operands, no `leading_zeros` `$clog2` outside the homes) and a lane op-set unit test; risk low to medium
* note: D12: PARSE × 68 + 23 × 46 × 39 share hdl-parser; PARSE × Q-NETSHAPE 0
* note: it frees rows 23 and 68 in one place (row 23 admits `<<`, `>>`, `**` in the parser lane, OPS already has them), row 46's B1 and j1a3 (§5.2 46)
* note: the home rows' trip wires and their honest cost are written once, in §5.2 SYS-1
* source: W/program/reorder2-2026-11-09/PLAN.txt G1 PARSE, OPS, G2 PARSE-1
