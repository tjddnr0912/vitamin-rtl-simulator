# s580 REPORT — inside: constant wildcard element compared with == instead of ==?

branch fix/inside-wildcard, HEAD 00c3d76d963f28e4ecc403bd5eba0807abcdbb40

## Step 0 PRE freeze
- command: `cargo build --release -p cli --locked` rc=0 (1m01s)
- PRE = s580/pre/vita  md5=9d37b3cdbd7f325fe273241dfbb3f465  size=7304752  built from HEAD 00c3d76d963f28e4ecc403bd5eba0807abcdbb40 (clean tree; only ?? .DS_Store)

## Step 1 reproduce at HEAD (PRE binary)
- `s580/pre/vita repro_inside_op.sv` rc=0: a 1, b x, c x, d 0, e x, f 1, g else-branch, h 1, i x — identical to the report's vita column.
- verilator 5.052 (`--binary --timing -DTWO_STATE`): a1 b1 c1 d0 e1 f1 g if-branch h1.
- sv2v v0.0.13 (s580/sv2v/sv2v-macOS/sv2v, md5 ca3f9b7138513e09a88be96aa7b5ea19) → iverilog 13.0: a1 b1 c1 d0 e1 f1 g if-branch h1 i x.
  sv2v spells a CONSTANT x/z element `(v | wildmask) == pattern_with_wild_as_1` (e.g. `(v | 4'b0100) == 4'b1100`),
  a plain element `v == 4'b1100`, several elements `|{...}`; a VARIABLE (and an overridable `parameter`) element as a runtime
  4-state formula `((e ^ (v ^ v)) === (v ^ (e ^ e))) & ((((v ^ v) ^ (e ^ e)) === (e ^ e)) | 1'bx)`; a fill element crashes
  sv2v (`Wildcard.hs:96 Non-exhaustive patterns`). iverilog 13.0 itself supports `==?` natively incl. a variable rhs
  (`v ==? e` with e=4'b1x00 → 1), so the `==?` twin of every single-element cell is a native 4-state iverilog oracle.
- iverilog 13.0 direct: `sorry: "inside" expressions not supported yet.` (every inside).

## Step 2 census
Files: s580/census/{A,W,S,X,V}.sv (+ `_2s` verilator twins, `_wq` ==? twins), census/e/E*.sv, census/c/C*.sv; raw outputs beside each (`*.pre.txt` vita PRE, `*.iv.txt` sv2v→iverilog, `*.ivd.txt` iverilog direct on ==? twins, `*.vl.txt` verilator).

### (a) positions `inside` reaches and the lowering each takes (measured PRE → POST, cells in census/c, census/z)
| position | lowering path | PRE (x/z element) | POST | oracle |
|---|---|---|---|---|
| procedural expr / `$display` (A*, W*, S*, X*) | `lower_expr` → expr_main `Binary` arm (fill-bearing → `lower_expr_ctx` `Binary` arm) | x | `==?` value | sv2v→iv + iv-direct `==?` + verilator |
| `if` (C01, report g) | stmt cond → `lower_expr` | else | then | both |
| ternary (C02) / always @* ternary (Z07) | `lower_expr` | 0X / X | 01 / 7 | both |
| continuous assign 1-bit / 8-bit (C03, C04, C23) | `lower_expr_ctx` | x / 0000000x / X | 1 / 00000001 / 1 | both |
| always_comb (C20), while (C21), for (C22), case item expr (C14), queue elem LHS (C32) | `lower_expr` | x / 1000 / 0 / default / x | 1 / 1001 / 2 / item / 1 | both (C32: verilator; sv2v cannot parse `$`) |
| `wait (…)` (C15) | event/wait lowering → `lower_expr` | never wakes | woke 1 | verilator (sv2v parse error) |
| immediate assert (C16) / concurrent property (C17, C24) | sva lowering → `lower_expr` | fail / fail 1,3,5 / end | pass / end / end | verilator; C17 also sv2v→iv |
| automatic function — route `frame` (C05), static function — route `inlined` (C06), task — `frame` (C07), function in cont. assign — `frame` (C08), static fns in cont. assign — `inlined` (C34), package fn — `frame` (C25), class method — uncounted (C26) | frame body / inline substitution, both via the same `Binary` arms; routes read from `--obs-dir` run.json `subroutines.items[].route` | x | 1 | verilator (+sv2v→iv except C26) |
| executors interp / vm / native | IR is executor-independent (`BitAnd`+`Eq` of consts) | — | 94 file×backend pairs byte-identical to native | — |
| JIT | `jit` feature off by default — not built, not measured | | | |
| localparam / parameter init (C10, C11, C12) | const domain: `const_compare_special` (i64 masked compare of a sized unsigned literal), `fold_region` (wide 4-state) | E3009 | 1 | both |
| generate-if (C13), generate-for if (C31) | same const domain | E3010 | then / in,in | both |
| range bound `[(… inside …):0]` (C28) | const domain | **1 (silent-wrong)** | 2 | both |
| instance override `#(.P(… inside …))` (C29) | override fold | E3009 | 1 | both |
| constant function body (C09, C30) | const-fn interpreter (`const_fn.rs`, i64) | E3009 (`cf(…)` has no constant-fold arm) | E3009 (unchanged; `==?` twin is E3009 too) | both 1 |
| localparam with an x LHS (C27) | const domain | E3009 | E3009 (unchanged) | both x |
| constraint `x inside {4'b1?00}` (C18), `randomize() with` (C19) | `crv.rs` i64 predicate | E3009 | E3009 (unchanged) | verilator FAILS to randomize (ok=0) — no oracle |
| constraint without x/z (`x inside {3,5,[10:12]}`, N01) | `map_cbinop` / `apply_cmp_bound` / `single_range_field` / `expr_field_interval` | works | works, same draws | — |
| event control `@(v inside …)` (C33) | event lowering | E3009 (bare signal only) | same | both run it |
| SVA property body inside a sequence (C24) | sva | end (no fail) | end | both |
| case `(e) inside` | parser does not accept it (§3.b case-inside) — out of scope | | | |

### (b) every `hdl_ast::BinOp` site (census by a read-only subagent, verified by compile errors + reading) — handling in this slice
Exhaustive (compile errors on the new variant): `expr_ctx.rs` `map_binop` (InsideEq → ir `Eq`, no debug_assert because `expr_size_ctx.rs:728` maps every comparison), `const_fn_width.rs` `binop_result_is_context_determined` (false), `const_level_header.rs` `binop_text` ("inside").
Catch-all / list sites given InsideEq beside Eq (SAME-AS-EQ, element without x/z is `==`):
- 1-bit result lists: `expr_ctx.rs` `ctx_widening_below`, `ast_expr_self_width`, `lower_expr_ctx` `is_cmp`; `expr_size_ctx.rs` `size_ctx_self_width`; `hoist/arms.rs` `arm_coercion_info`; `stmt_flow.rs` `case_operand_takes_ctx`; `override_type.rs` `override_top_meta`; `const_wide.rs` `wide_top_is_self_determined`; hdl-parser `packed_md.rs` `ctx_invariant`; hdl-parser `typedefs.rs` `names_an_overridable`.
- handle gate `is_eq` and StrCmp route: `expr_main.rs` (both) and `expr_ctx.rs` `binary_stops_ctx` (verbatim twin).
- i64/f64 folds where an x/z literal cannot arrive (the leaf fold declines): `const_fn.rs` `const_binop`, `const_real.rs`, `const_eval.rs` `delay_units_in_scope`, `block_local/proofs.rs` `fold_i32` (decimal leaves only).
- constraints, moved together (netdecl alone would drop the constraint): `crv.rs` `map_cbinop`, `apply_cmp_bound`, `apply_constraint_expr`; `netdecl.rs` `single_range_field`; `expr_special.rs` `expr_field_interval` (both lists).
- string const compare: `const_str.rs` `const_compare_special` string arm + `want_eq`.
- diagnostics text: `const_bound.rs` `bin_op_text` ("inside").
WILD-AWARE sites:
- `expr_main.rs` generic `Binary` arm and `expr_ctx.rs` `lower_expr_ctx` generic comparison arm: after both operands are lowered (unchanged calls), `inside_value_cmp(el_ast, lhs_id, el_id)`.
- `const_str.rs` `const_compare_special`: InsideEq with a SIZED UNSIGNED literal carrying x/z takes the existing `==?` masked compare; a signed one returns None (generic fold declines on the literal → loud).
- `const_wide.rs` `fold_region`: InsideEq with an x/z element DECLINES (this 4-state domain has no `==?`); `wide_eq_with_unknowns` and the `ord` sense list take InsideEq beside Eq (reached only with a known element).
NO-CHANGE (catch-all already gives Eq's answer, Eq is not listed there either): `expr_ctx.rs` `binop_is_widening`; `expr_size_ctx.rs` `is_size_ctx_operation`, `ctx_signed_impl` (`_ => Some(false)`); `const_eval.rs` `const_expr_signed`; `const_fn_width.rs` `const_self_width`, `const_signed_env`; `const_bound.rs` `ast_has_width_growing_op`, `unfoldable_reason_in` (WildEq+string reason only); `const_wide_num.rs`; `cover_bins.rs`; `param_query.rs`; `frames_classify.rs`; `inline_body_ctx.rs`; `expr_size_hier*.rs`; `hoist/general.rs` `shape` (`_ => Uncond`); every LogAnd/LogOr-only site; hdl-parser arith-only folds; constructors (`cover_synth.rs` keeps Eq for bins, sva_*, enums.rs, stmt_ctl foreach guard).
Serialization: no site uses BinOp's discriminant; postcard `.vu` discriminants of existing variants unchanged (appended last); `crates/hdl-ast/tests/schema_hash.rs` EXPECTED re-pinned; no other golden embeds the hdl-ast hash; `.vu` uses the shared `CURRENT_FORMAT_VERSION` (34) whose gate is the schema hash — every prior hdl-ast re-pin left format_version unchanged; sim-ir untouched.

### (c) element shapes — what each lowers to and its value (PRE → POST), census/e
- sized literal with x/z/? anywhere incl. MSB (A07–A14, A29–A32 incl. `4'hx`, `4'dx`, `4'o1?`): IR `Const` with unk bits → PRE `x`, POST `==?` (all = both oracles).
- unsized based (`'b1?00` E01, `'bx1` E02/E03 4-bit LHS, `'b?` E04, `'h?` E07): `Const` 32-bit → POST right. `'bx1` against a 36-bit LHS (E05): PRE 0, POST 1 = iverilog's own `==?` (E05q 1) and verilator 1; sv2v→iverilog 0 is sv2v's literal rewrite (`'b111…10` unsized) — disqualified per ER §7.3 row "Run iverilog itself before calling a literal's reading two-oracle".
- fill `'x`/`'z` (E10, E11, E15): sized to the LHS by the ctx path → `Const` all-unk → POST 1 (verilator 1, iverilog `==?` twin 1; sv2v crashes on fills). `'1`/`'0` (E12–E14): no x/z → `==` unchanged.
- parameter / localparam holding x/z (E20–E23, E26), `'x` fill localparam (E24), enum label `4'b1x00` (E30): LOUD at the DECLARATION on PRE and POST (E3009) — pre-existing, the element never lowers. x/z-free parameter (E25) / enum (E31): unchanged.
- compound const with x/z: concat (E40), `P | 4'b000x` (E41), size cast (E43), `?:` (E44), replicate (E45), `~` (E46), `$unsigned` (E47), runtime concat `{p, 2'b?0}` (E82): PRE x (E45 PRE 0 = right by a definite mismatch), POST LOUD E3009 "an `inside` element that builds an x/z/? literal into a larger expression…". Parenthesised literal (E42): `Const` → POST 1.
- constant function call returning `4'b1?00` (E50): IR `Call` → `==` (runtime residue) PRE x POST x, sv2v→iv 1, verilator refuses.
- const tree producing x from known leaves `4'd1/4'd0` (E88): `==` residue, PRE x POST x, iverilog `==?` twin 1, verilator 1.
- string literal (E60, E61), real (E62–E65): unchanged (`==`, non-integral or no x/z).
- 2-state variable element (V03), 4-state variable element (V01, V02, V04, V05, V07, V08, V10): `Signal` → `==`; with an x/z VALUE at run time V01/V04/V08/V10 stay x (sv2v runtime formula → iverilog 1; iverilog direct `v ==? e` 1). Residue.
- unpacked array name (E70): E3009 "a whole unpacked array has no value" PRE and POST (measure only; sv2v→iverilog refuses too, verilator 1).
- struct member as element (E71, variable): `==`, unchanged. struct member / array element / select / concat / arithmetic / `$unsigned` / `$signed` as LHS (E72, E73, E78–E80, E85–E87): POST right.

### (d) oracle census matrix — raw per-cell values (POST = debug build of the working tree at the time of the run)
Columns: vita PRE | vita POST | sv2v 0.0.13 → iverilog 13.0 | verilator 5.052 (2-state cells only; X group = n/a) | iverilog 13.0 direct on the `==?` twin of a single-element cell | PRE and POST on that `==?` twin.

| id | cell | PRE | POST | sv2v→iv | verilator | iv-direct `==?` twin | PRE `==?` twin | POST `==?` twin |
|---|---|---|---|---|---|---|---|---|
| A01 | `v=4'b1100; v inside {4'b1100}` | 1 | 1 | 1 | 1 | 1 | 1 | 1 |
| A02 | `v=4'b1100; v inside {4'b1?00}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A03 | `v=4'b1000; v inside {4'b1?00}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A04 | `v=4'b0100; v inside {4'b1?00}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| A05 | `v=4'b1100; v inside {4'b0000, 4'b1?00}` | x | 1 | 1 | 1 |  |  |  |
| A06 | `v=4'b0101; v inside {[4'd1:4'd7]}` | 1 | 1 | 1 | 1 |  |  |  |
| A07 | `v=4'b1100; v inside {4'b1x00}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A08 | `v=4'b1100; v inside {4'b1z00}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A09 | `v=4'b0100; v inside {4'b?100}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A10 | `v=4'b1100; v inside {4'bx100}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A11 | `v=4'b0100; v inside {4'bz100}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A12 | `v=4'b1101; v inside {4'b110?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A13 | `v=4'b1010; v inside {4'b????}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A14 | `v=4'b1010; v inside {4'bxxxx}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A15 | `v=4'b0101; v inside {4'b1?00, [4'd1:4'd7]}` | 1 | 1 | 1 | 1 |  |  |  |
| A16 | `v=4'b1000; v inside {[4'd1:4'd7], 4'b1?00}` | x | 1 | 1 | 1 |  |  |  |
| A17 | `v=4'b0011; v inside {[4'd1:4'd2], 4'b1?00}` | 0 | 0 | 0 | 0 |  |  |  |
| A18 | `v=4'b1100; !(v inside {4'b1?00})` | x | 0 | 0 | 0 |  |  |  |
| A19 | `v=4'b0100; !(v inside {4'b1?00})` | 1 | 1 | 1 | 1 |  |  |  |
| A20 | `v=4'b1100; v inside {3'b1?0}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| A21 | `v=4'b0110; v inside {3'b1?0}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A22 | `v=4'b1100; v inside {6'b001?00}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A23 | `v=4'b1100; v inside {6'b101?00}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| A24 | `v=4'b1100; v inside {6'b??1?00}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A25 | `v=4'b0110; v inside {4'b0000, 4'b0001, 4'b01?0}` | x | 1 | 1 | 1 |  |  |  |
| A26 | `v=4'b0110; v inside {4'b01?0, 4'b0000}` | x | 1 | 1 | 1 |  |  |  |
| A27 | `v=4'b0111; v inside {4'b01?0, 4'b0000}` | 0 | 0 | 0 | 0 |  |  |  |
| A28 | `v=4'b1100; v inside {4'hC}` | 1 | 1 | 1 | 1 | 1 | 1 | 1 |
| A29 | `v=4'b1100; v inside {4'h?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A30 | `v=4'b1100; v inside {4'hx}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A31 | `v=4'b1100; v inside {4'dx}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A32 | `v=4'b1100; v inside {4'o1?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| A33 | `v=4'b0100; v inside {4'b1?00} || v inside {4'b0?00}` | x | 1 | 1 | 1 |  |  |  |
| A34 | `v=4'b0100; (v inside {4'b1?00}) + 2'd1` | 01 | 01 | 01 | 01 |  |  |  |
| A35 | `v=4'b1100; {v inside {4'b1?00}, v inside {4'b0?00}}` | x0 | 10 | 10 | 10 |  |  |  |
| A36 | `v=4'b1100; 4'b1100 inside {4'b1?00}` | x | 1 | 1 | 1 |  |  |  |
| A37 | `v=4'b1100; 4'b0100 inside {4'b1?00}` | 0 | 0 | 0 | 0 |  |  |  |
| W8a | `u8=8'b01111110; u8 inside {8'b?111111?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W8b | `u8=8'b11111111; u8 inside {8'b?111111?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W8c | `u8=8'b11111101; u8 inside {8'b?111111?}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| W8d | `u8=8'b11111111; u8 inside {4'b1?11}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| W8e | `u8=8'b1011; u8 inside {4'b1?11}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W16a | `u16=16'b0111111111111110; u16 inside {16'b?11111111111111?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W16b | `u16=16'b1111111111111111; u16 inside {16'b?11111111111111?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W16c | `u16=16'b1111111111111101; u16 inside {16'b?11111111111111?}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| W16d | `u16=16'b1111111111111111; u16 inside {4'b1?11}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| W16e | `u16=16'b1011; u16 inside {4'b1?11}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W32a | `u32=32'b01111111111111111111111111111110; u32 inside {32'b?111111111111111111111111111111?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W32b | `u32=32'b11111111111111111111111111111111; u32 inside {32'b?111111111111111111111111111111?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W32c | `u32=32'b11111111111111111111111111111101; u32 inside {32'b?111111111111111111111111111111?}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| W32d | `u32=32'b11111111111111111111111111111111; u32 inside {4'b1?11}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| W32e | `u32=32'b1011; u32 inside {4'b1?11}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W33a | `u33=33'b011111111111111111111111111111110; u33 inside {33'b?1111111111111111111111111111111?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W33b | `u33=33'b111111111111111111111111111111111; u33 inside {33'b?1111111111111111111111111111111?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W33c | `u33=33'b111111111111111111111111111111101; u33 inside {33'b?1111111111111111111111111111111?}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| W33d | `u33=33'b111111111111111111111111111111111; u33 inside {4'b1?11}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| W33e | `u33=33'b1011; u33 inside {4'b1?11}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W64a | `u64=64'b0111111111111111111111111111111111111111111111111111111111111110; u64 inside {64'b?11111111111111111111111111111111111111111111111111111111111111?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W64b | `u64=64'b1111111111111111111111111111111111111111111111111111111111111111; u64 inside {64'b?11111111111111111111111111111111111111111111111111111111111111?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W64c | `u64=64'b1111111111111111111111111111111111111111111111111111111111111101; u64 inside {64'b?11111111111111111111111111111111111111111111111111111111111111?}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| W64d | `u64=64'b1111111111111111111111111111111111111111111111111111111111111111; u64 inside {4'b1?11}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| W64e | `u64=64'b1011; u64 inside {4'b1?11}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W65a | `u65=65'b01111111111111111111111111111111111111111111111111111111111111110; u65 inside {65'b?111111111111111111111111111111111111111111111111111111111111111?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W65b | `u65=65'b11111111111111111111111111111111111111111111111111111111111111111; u65 inside {65'b?111111111111111111111111111111111111111111111111111111111111111?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W65c | `u65=65'b11111111111111111111111111111111111111111111111111111111111111101; u65 inside {65'b?111111111111111111111111111111111111111111111111111111111111111?}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| W65d | `u65=65'b11111111111111111111111111111111111111111111111111111111111111111; u65 inside {4'b1?11}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| W65e | `u65=65'b1011; u65 inside {4'b1?11}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W128a | `u128=128'b01111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111110; u128 inside {128'b?111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W128b | `u128=128'b11111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111; u128 inside {128'b?111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111?}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| W128c | `u128=128'b11111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111101; u128 inside {128'b?111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111?}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| W128d | `u128=128'b11111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111111; u128 inside {4'b1?11}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| W128e | `u128=128'b1011; u128 inside {4'b1?11}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| S01 | `s8=-8'sd4; s8 inside {4'sb1?00}` | x | 1 | 1 | 1 | 1 | 0 | 1 |
| S02 | `s8=-8'sd4; s8 inside {4'sb?100}` | x | 1 | 1 | 0 | 1 | 0 | 1 |
| S03 | `s8=8'sb01010100; s8 inside {4'sb?100}` | x | 1 | 1 | 0 | 1 | 0 | 1 |
| S04 | `s8=-8'sd4; s8 inside {4'b1?00}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| S05 | `s8=-8'sd4; s8 inside {8'sb11111?00}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| S06 | `u8=8'b11111100; u8 inside {4'sb1?00}` | 0 | 0 | 0 | 0 | 0 | 0 | 0 |
| S07 | `s8=-8'sd4; s8 inside {-8'sd4}` | 1 | 1 | 1 | 1 |  |  |  |
| S08 | `s8=-8'sd4; s8 inside {4'sb1100}` | 1 | 1 | 1 | 1 | 1 | 0 | 1 |
| S09 | `s4=-4'sd4; s4 inside {8'sb11111?00}` | x | 1 | 1 | 1 | 1 | 0 | 1 |
| S10 | `s4=-4'sd4; s4 inside {8'b11111?00}` | 0 | 0 | 0 | 1 | 0 | 0 | 0 |
| S11 | `s4=-4'sd4; s4 inside {8'sb00001?00}` | 0 | 0 | 0 | 0 | 0 | 1 | 0 |
| S12 | `s8=8'sb00001100; s8 inside {4'sb1?00}` | 0 | 0 | 0 | 0 | 0 | 1 | 0 |
| S13 | `s8=8'sb00001100; s8 inside {4'b1?00}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| S14 | `i32=-4; i32 inside {4'sb1?00}` | x | 1 | 1 | 1 | 1 | 0 | 1 |
| S15 | `i32=-4; i32 inside {32'sb1111111111111111111111111111?100}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| S16 | `s8=-8'sd4; s8 inside {4'sbx100}` | x | 1 | 1 | 0 | 1 | 0 | 1 |
| S17 | `s8=8'sb01010100; s8 inside {4'sbz100}` | x | 1 | 1 | 0 | 1 | 0 | 1 |
| S18 | `s8=-8'sd4; s8 inside {4'sb1100, 4'sb1?00}` | 1 | 1 | 1 | 1 |  |  |  |
| S19 | `s4=-4'sd4; s4 inside {8'sb?1111100}` | x | 1 | 1 | 1 | 1 | 0 | 1 |
| S20 | `s4=4'sb0100; s4 inside {8'sb?0000100}` | x | 1 | 1 | 1 | 1 | 1 | 1 |
| S21 | `s8=-8'sd4; s8 inside {[-8'sd5:-8'sd3]}` | 1 | 1 | 1 | 1 |  |  |  |
| S22 | `s8=-8'sd4; s8 inside {[-8'sd5:-8'sd3], 4'sb0?00}` | 1 | 1 | 1 | 1 |  |  |  |
| X01 | `v=4'b1x00; v inside {4'b1?00}` | x | 1 | 1 | n/a (x LHS) | 1 | 1 | 1 |
| X02 | `v=4'bx100; v inside {4'b1?00}` | x | x | x | n/a (x LHS) | x | x | x |
| X03 | `v=4'bz100; v inside {4'b1?00}` | x | x | x | n/a (x LHS) | x | x | x |
| X04 | `v=4'b1z00; v inside {4'b1?00}` | x | 1 | 1 | n/a (x LHS) | 1 | 1 | 1 |
| X05 | `v=4'bx100; v inside {4'b0000, 4'b1?00}` | x | x | x | n/a (x LHS) |  |  |  |
| X06 | `v=4'bx100; v inside {4'b?100, 4'b1?00}` | x | 1 | 1 | n/a (x LHS) |  |  |  |
| X07 | `v=4'bx100; v inside {4'b?100}` | x | 1 | 1 | n/a (x LHS) | 1 | 1 | 1 |
| X08 | `v=4'bxxxx; v inside {4'b????}` | x | 1 | 1 | n/a (x LHS) | 1 | 1 | 1 |
| X09 | `v=4'bx100; v inside {4'b0100}` | x | x | x | n/a (x LHS) | x | x | x |
| X10 | `v=4'bx100; v inside {[4'd1:4'd7]}` | x | x | x | n/a (x LHS) |  |  |  |
| X11 | `v=4'b1x00; v inside {4'b1x00}` | x | 1 | 1 | n/a (x LHS) | 1 | 1 | 1 |
| X12 | `v=4'bx100; v inside {4'b0?00}` | x | x | x | n/a (x LHS) | x | x | x |
| X13 | `v=4'bx110; v inside {4'b0?00}` | 0 | 0 | 0 | n/a (x LHS) | 0 | 0 | 0 |
| X14 | `v=4'bx100; v inside {4'b1?00, 4'b0110}` | x | x | x | n/a (x LHS) |  |  |  |
| X15 | `v=4'bx100; !(v inside {4'b1?00})` | x | x | x | n/a (x LHS) |  |  |  |
| X16 | `v=4'bzzzz; v inside {4'bzzzz}` | x | 1 | 1 | n/a (x LHS) | 1 | 1 | 1 |
| X17 | `v=4'bzzzz; v inside {4'b0000}` | x | x | x | n/a (x LHS) | x | x | x |
| X18 | `v=4'bx100; v inside {6'b??1?00}` | x | x | x | n/a (x LHS) | x | x | x |
| X19 | `v=4'bx100; v inside {6'b011?00}` | 0 | 0 | 0 | n/a (x LHS) | 0 | 0 | 0 |
| X20 | `v=4'b1100; v inside {4'b1?00, 4'bx}` | x | 1 | 1 | n/a (x LHS) |  |  |  |
| V01 | `v=4'b1100; e=4'b1x00; v inside {e}` | - | - | 1 | - |  |  |  |
| V02 | `v=4'b1100; e=4'b1100; v inside {e}` | - | - | 1 | - |  |  |  |
| V03 | `v=4'b1100; be=4'b1100; v inside {be}` | - | - | 1 | - |  |  |  |
| V04 | `v=4'b1000; e=4'b1z00; v inside {e}` | - | - | 1 | - |  |  |  |
| V05 | `v=4'b1x00; e=4'b1100; v inside {e}` | - | - | x | - |  |  |  |
| V06 | `v=4'b1100; e=4'b1x00; v ==? e` | - | - | 1 | - |  |  |  |
| V07 | `v=4'b0100; e=4'b1x00; v inside {e}` | - | - | 0 | - |  |  |  |
| V08 | `v=4'b1100; e=4'b1x00; v inside {4'b0000, e}` | - | - | 1 | - |  |  |  |
| V09 | `v=4'b1100; e=4'b0000; v inside {e, 4'b1?00}` | - | - | 1 | - |  |  |  |
| V10 | `v=4'b1100; e=4'bxxxx; v inside {e}` | - | - | 1 | - |  |  |  |

Vr (V without the loud V06 line): PRE/POST/sv2v→iv: V01 x/x/1 · V02 1/1/1 · V03 1/1/1 · V04 x/x/1 · V05 x/x/x · V07 0/0/0 · V08 x/x/1 · V09 x/1/1 · V10 x/x/1

| id | first probed expr | PRE | POST | sv2v→iv | verilator |
|---|---|---|---|---|---|
| E01 | `v inside {'b1?00}` | x | 1 | 1 | 1 |
| E02 | `v inside {'bx1}` | x | 1 | 1 | 1 |
| E03 | `v inside {'bx1}` | 0 | 0 | 0 | 0 |
| E04 | `v inside {'b?}` | x | 1 | 1 | 1 |
| E05 | `v inside {'bx1}` | 0 | 1 | 0 | 1 |
| E06 | `v inside {'b1x1}` | 0 | 0 | 0 | 0 |
| E07 | `v inside {'h?}` | x | 1 | 1 | 1 |
| E10 | `v inside {'x}` | x | 1 |  | 1 |
| E11 | `v inside {'z}` | x | 1 |  | 1 |
| E12 | `v inside {'1}` | 1 | 1 | 1 | 1 |
| E13 | `v inside {'1}` | 0 | 0 | 0 | 0 |
| E14 | `v inside {'0}` | 1 | 1 | 1 | 1 |
| E15 | `v inside {4'b0001, 'x}` | x | 1 |  | 1 |
| E20 | `v inside {L}` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 | 1 |
| E21 | `v inside {L}` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 | 1 |
| E22 | `v inside {P}` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 | 1 |
| E23 | `v inside {L}` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 | 1 |
| E24 | `v inside {L}` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 0 | 1 |
| E25 | `v inside {L}` | 1 | 1 | 1 | 1 |
| E26 | `v ==? L` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 | 1 |
| E30 | `v inside {A}` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 | 1 |
| E31 | `v inside {A, B}` | 1 | 1 | 1 | 1 |
| E40 | `v inside {{2'b1?, 2'b00}}` | x | LOUD error[VITA-E3009] | 1 | 1 |
| E41 | `v inside {P \| 4'b000x}` | x | LOUD error[VITA-E3009] | 1 | 1 |
| E42 | `v inside {(4'b1?00)}` | x | 1 | 1 | 1 |
| E43 | `v inside {4'(4'b1?00)}` | x | LOUD error[VITA-E3009] | 1 | 1 |
| E44 | `v inside {1'b1 ? 4'b1?00 : 4'b0000}` | x | LOUD error[VITA-E3009] | 1 | 1 |
| E45 | `v inside {{2{2'b?0}}}` | 0 | LOUD error[VITA-E3009] | 0 | 0 |
| E46 | `v inside {~4'b0?11}` | x | LOUD error[VITA-E3009] | 1 | 1 |
| E47 | `v inside {$unsigned(4'b1?00)}` | x | LOUD error[VITA-E3009] | 1 | 1 |
| E50 | `v inside {cf()}` | x | x | 1 | LOUD %Error-UNSUPPORTED; LOUD %Error |
| E60 | `s16 inside {"ab"}` | 1 | 1 | 1 | 1 |
| E61 | `s inside {"ab", "cd"}` | 1 | 1 |  | 1 |
| E62 | `r inside {1.5}` | 1 | 1 |  | 1 |
| E63 | `r inside {[1.0:2.0]}` | 1 | 1 | 1 | 1 |
| E64 | `v inside {2.0}` | 1 | 1 |  | 1 |
| E65 | `r inside {4'b1?00}` | 0 | 0 |  | 0 |
| E70 | `v inside {arr}` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] |  | 1 |
| E71 | `v inside {s.a}` | 1 | 1 | 1 | LOUD %Error-UNSUPPORTED; LOUD %Error |
| E72 | `s.a inside {4'b1?00}` | x | 1 | 1 | 1 |
| E73 | `arr[1] inside {4'b1?00}` | x | 1 | 1 | 1 |
| E74 | `bv inside {4'b1?00}` | x | 1 | 1 | 1 |
| E75 | `iv inside {4'b1?00}` | x | 1 | 1 | 1 |
| E76 | `b` | 0 | 1 | 1 | 1 |
| E77 | `w8` | 0000000x | 00000001 | 00000001 | 00000001 |
| E78 | `v[3:0] inside {4'b1?00}` | x | 1 | 1 | 1 |
| E79 | `{v[3:2], v[1:0]} inside {4'b1?00}` | x | 1 | 1 | 1 |
| E80 | `(v + 4'd0) inside {4'b1?00}` | x | 1 | 1 | 1 |
| E81 | `v inside {e, 4'b0?00}` | 1 | 1 | 1 | LOUD %Error-UNSUPPORTED; LOUD %Error |
| E82 | `v inside {{p, 2'b?0}}` | x | LOUD error[VITA-E3009] | 1 | LOUD %Error-UNSUPPORTED; LOUD %Error |
| E83 | `v inside {4'sb1?00}` | x | 1 | 1 | 1 |
| E84 | `v inside {4'b1?00}` | x | 1 | 1 | 1 |
| E85 | `$unsigned(v) inside {4'b1?00}` | x | 1 | 1 | 1 |
| E86 | `$signed(v[3:0]) inside {8'sb1111?100}` | 0 | 0 | 0 | 0 |
| E87 | `$signed(v[3:0]) inside {4'sb?100}` | x | 1 | 1 | 1 |
| E88 | `v inside {4'd1/4'd0}` |  | x |  |  |
| C01 | `` | else | then | then | then |
| C02 | `(v inside {4'b1?00}) ? 8'd1 : 8'd2` | 0X | 01 | 01 | 01 |
| C03 | `w` | x | 1 | 1 | 1 |
| C04 | `w8` | 0000000x | 00000001 | 00000001 | 00000001 |
| C05 | `f(v)` | x | 1 | 1 | 1 |
| C06 | `f(v)` | x | 1 | 1 | 1 |
| C07 | `r` | x | 1 | 1 | 1 |
| C08 | `w` | x | 1 | 1 | 1 |
| C09 | `L` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 | 1 |
| C10 | `L1` | LOUD error[VITA-E3009] | 1 | 1 | 1 |
| C11 | `L2` | LOUD error[VITA-E3009] | 1 | 1 | 1 |
| C12 | `L3` | LOUD error[VITA-E3009] | 1 | 1 | 1 |
| C13 | `` | LOUD error[VITA-E3010] | then | then | then |
| C14 | `` | default | item | item | item |
| C15 | `$time` | end | woke 1; end |  | woke 1; end |
| C16 | `` | fail | pass |  | pass |
| C17 | `$time` | fail 1; fail 3; fail 5; end | end | end | end |
| C18 | `ok, o.x` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] |  | 0 0000; 0 0000; 0 0000; 0 0000; 0 0000; 0 0000 |
| C19 | `ok, o.x` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] |  | 0 1000; 0 0011; 0 0011; 0 0110; 0 1001; 0 0011 |
| C20 | `m` | x | 1 | 1 | 1 |
| C21 | `v` | 1000 | 1001 | 1001 | 1001 |
| C22 | `n` | 0 | 2 | 2 | 2 |
| C23 | `y` | X | 1 | 1 | 1 |
| C24 | `$time` | end | end | end | end |
| C25 | `p::pf(v)` | x | 1 | 1 | 1 |
| C26 | `k.m(v)` | x | 1 |  | 1 |
| C27 | `L4` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | x | x |
| C28 | `$bits(w)` | 1 | 2 | 2 | 2 |
| C28b | `$bits(w), $bits(w2), $bits(w3), $bits(w4)` |  | 1 2 2 1 |  |  |
| C29 | `P` | LOUD error[VITA-E3009] | 1 | 1 | 1 |
| C30 | `f(v), LC` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 1 | 1 1 |
| C31 | `i` | LOUD error[VITA-E3010]; LOUD error[VITA-E3010] | i=0 in; i=1 in | i=0 in; i=1 in | i=0 in; i=1 in |
| C32 | `q[0] inside {4'b1?00}` | x | 1 |  | 1 |
| C33 | `` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | edge; end | edge; end |
| C34 | `w8, f(v), g(v)` | xx x xx; xx x 01 | 03 1 02; 02 1 01 |  | 03 1 02; 02 1 01 |
| Z01 | `u8 inside {4'bx100}` | Z01a x; Z01b 0; Z01c x; Z01d x; Z01e 0; Z01f x; Z01g 0; Z01h x; Z01i x; Z01j x; Z01k xxxx; Z01l x | Z01a 1; Z01b 0; Z01c 1; Z01d 1; Z01e 0; Z01f 1; Z01g 0; Z01h 1; Z01i 1; Z01j 0; Z01k 1111; Z01l 3 | Z01a 1; Z01b 0; Z01c 1; Z01d 1; Z01e 0; Z01f 1; Z01g 0; Z01h 1; Z01i 1; Z01j 0; Z01k 1111; Z01l 3 | Z01a 1; Z01b 0; Z01c 1; Z01d 1; Z01e 0; Z01f 1; Z01g 0; Z01h 1; Z01i 1; Z01j 0; Z01k 1111; Z01l 3 |
| Z02 | `u.sig inside {4'sb1?00}` | x | 1 | 1 | 1 |
| Z03 | `h inside {null, h2}, h2 inside {null}` | 1 0 | 1 0 |  | 1 0 |
| Z04 | `L` | LOUD error[VITA-E3009] | LOUD error[VITA-E3009] | 1 | 0 |
| Z05 | `L, L2` | 0 1 | 0 1 | 1 0 | 0 0 |
| Z06 | `i` |  | 0; 2 | 0; 2 | 0; 2 |
| Z07 | `r` |  X;  3 |  7;  3 |  7;  3 |  7;  3 |

`==?` twins of element shapes (census/e/Q1.sv; PRE | POST | iverilog direct): E01q 1|1|1 · E02q 1|1|1 · E03q 0|0|0 · E04q 1|1|1 · **E05q 0|1|1** · E06q 0|0|0 · E05r 1|1|1 · E05s 1|1|1 · E07q 1|1|1 · E10q 1|1|1 · E11q 1|1|1 · E15q 1|1|1.

### Divergence classification (4-way) — every PRE/oracle divergence in the census
- **real gap, closed by this slice (PRE silent-wrong → POST = oracle)**: A02 A03 A05 A07–A14 A16 A18 A21 A22 A24–A26 A29–A33 A35 A36; W{8,16,32,33,64,65,128}{a,b,e}; S01 S02 S03 S05 S09 S13 S14 S15 S16 S17 S19 S20; X01 X04 X06 X07 X08 X11 X16 X20; V09; E01 E02 E04 E05 E07 E10 E11 E15 E42 E72–E80 E83–E85 E87; C01–C08 C14–C17 C20–C23 C25 C26 C28 C32 C34; Z01 Z02 Z06 Z07; report cells b c e g.
- **real gap, loud→value (PRE E3009/E3010 → POST = both oracles)**: C10 C11 C12 C13 C29 C31.
- **real gap → LOUD (PRE silent-wrong x → POST E3009)**: E40 E41 E43 E44 E46 E47 E82. **E45 `v inside {{2{2'b?0}}}` with v=1100: PRE 0 = both oracles (right only because bit 2 is a definite mismatch; v=0000 would be x where the oracles say 1) → POST loud — a correct→loud on a coincidence cell of a shape that is silent-wrong in general (prescribed: compound x/z → loud).**
- **pre-existing `==?` operator silent-wrongs, closed by the shared builder** (iverilog direct `==?` is the oracle; sv2v→iv agrees; verilator agrees except the 2-state cells): S01q S02q S03q S08q S09q S11q S12q S14q S16q S17q S19q, E05q. (PRE `==?` zero-extended a signed pattern and a signed narrower LHS, and did not x-extend an unsized x/z literal past 32 bits.)
- **residue, recorded, not fixed (silent-wrong remains)**: runtime x/z element — V01 V04 V08 V10 (4-state variable holding x/z; POST x, sv2v runtime formula → iv 1, iverilog direct `v ==? e` 1), E50 (function returning `4'b1?00`; POST x, sv2v→iv 1, verilator refuses), E88 (`4'd1/4'd0` element; POST x, iverilog `==?` twin 1, verilator 1). Const-domain `==?` operator with a SIGNED pattern Z05 (`S=8'sd84 ==? 4'sb?100` vita 0 / iverilog 1; `S2=8'sd12 ==? 4'sb1?00` vita 1 / iverilog 0, verilator 0) — pre-existing in `const_compare_special`, unchanged; InsideEq does not route a signed pattern there (Z04 stays loud).
- **residue, loud, unchanged**: C09 C30 (constant function body with an x/z literal), C27 (x LHS in a localparam), C18 C19 (constraint with x/z element — verilator fails to randomize, no oracle), E20–E24 E26 E30 (x/z parameter / enum label refused at the declaration), E70 (unpacked array element), C33 (expression event control), Z04 (signed x/z pattern in the const domain; sv2v→iv 1, verilator 0 split on the x sign bit, hand-IEEE 1).
- **verilator disqualified (2-state)**: S02 S03 S16 S17 (an x/z sign bit extends as x → don't-care; verilator reads it as 0 and answers 0; iverilog direct and sv2v→iv answer 1, hand-IEEE §11.4.5 + §11.4.6 = 1); all X cells (x LHS).
- **verilator own defect**: S10 `s4=-4 inside {8'b11111?00}` (unsigned element): verilator 1, iverilog direct 0 and sv2v→iv 0, hand-IEEE §11.4.5 (one operand unsigned → zero extension) 0; vita 0 PRE and POST.
- **harness-format**: E05 sv2v→iv 0 vs iverilog direct 1 (sv2v rewrites `'bx1` into a 32-bit unsized `'b11…10` mask); sv2v crashes on a fill element (E10 E11 E15 C34: `Wildcard.hs:96 Non-exhaustive patterns`); sv2v cannot parse `fork…join_none` (C15), `new` (C26, Z03), a queue `[$]` (C32); E62/E64/E65 sv2v emits `^`/`|` on a real.
- **vita-ahead**: none claimed.

## Step 3 design (as built)
- `hdl_ast::BinOp::InsideEq` appended LAST (postcard discriminants of existing variants unchanged); `parse_inside` emits it for every VALUE element (ranges unchanged, the duplicated-LHS comment kept).
- New module `crates/elaborate/src/wildcard_eq.rs` (lower_wildcard_eq moved out of expr_special.rs, which was 1033 lines → 921):
  - `inside_value_cmp(el_ast, lhs_id, el_id) -> Option<u32>`: operands already lowered once by the unchanged `Eq` calls of each `Binary` arm (expr_main generic arm, expr_ctx `lower_expr_ctx` comparison arm). Dispatch: real operand → None (`==`); `!expr_may_be_unknown(el)` → None; el is one `Const` (Numeric) with unk bits → `wildcard_cmp_ids`; else an x/z literal reaching the value (`xz_literal_reaches_value`, exhaustive over the 11 `ir::Expr` kinds) → LOUD E3009; else None (runtime residue). None ⇒ caller pushes the same `Eq` node as before ⇒ byte-identical IR.
  - `wildcard_cmp_ids(lhs_id, pat_ast, cv, ne)`: the one builder for `==?`/`!=?` and `inside`. New vs PRE only when (a) the pattern is SIGNED, of another width, and the left operand is signed (canonical_self_width): pattern sign-extends (x/z sign bit → don't-care) or, if the left operand is narrower, mask AND cleaned consts are signed so the engine sign-extends it; (b) an UNSIZED literal (AST) whose MSB is x/z: extension bits are don't-care (§5.7.1). Otherwise identical consts/nodes as PRE `lower_wildcard_eq`. Loud: unsizable LHS (existing), signed pattern of other width with unknown LHS sign (placeholder).
  - Deviation from the brief's "be loud where the mask would be wrong": the census showed the mask CAN be built right for the sign/unsized cases, and the `==?` operator (the existing caller of the same builder) was silently wrong on the same cells (S01q…S19q, E05q), so per ER §2.3 "when [existing callers] are wrong too, the fix belongs inside the helper" the builder was fixed for both, measured against iverilog 13.0's own `==?` (16 F16/S-group cells).
- Const domains: `const_compare_special` takes InsideEq with a SIZED UNSIGNED x/z literal through the existing `==?` masked compare; `fold_region` declines an x/z element; every other fold treats InsideEq as Eq (an x/z literal never reaches them: their leaf folds decline).
- Constraints: InsideEq → `CBinOp::Eq` together with the [lo,hi] narrowing and the single-field skip (x/z constants decline in `const_eval_in_scope` → E3009, as PRE).
- No MsgCode added (E3009 reused); no sim-ir change; `CURRENT_FORMAT_VERSION` unchanged; hdl-ast SchemaHash re-pinned (crates/hdl-ast/tests/schema_hash.rs).

## Step 4 tests — crates/cli/tests/inside_wildcard.rs (20 tests)
Teeth: the test executable run with target/debug/vita replaced by s580/pre/vita (fresh inode; restored, md5 verified): 16 FAILED, 4 ok — the 4 that pass on PRE are by design: `backends_agree` (equivalence), `constant_and_constraint_x_z_shapes_stay_loud` (guard: PRE loud too), `elements_without_x_z_are_unchanged` (control), `runtime_x_z_element_residue` (residue pinned at the observed value). Raw: s580/teeth_pre.txt. PRE output of every pinned design is quoted in each test's doc comment (`PRE (00c3d76d) printed: …`).

## Step 5 gate
- run1 (s580/gate/run1): fmt rc=0; clippy rc=101 — `clippy::precedence` at wildcard_eq.rs:247 (`x >> (i % 64) & 1 == 1`; same parse, parenthesized); nextest `-p cli -p elaborate -p hdl-parser -p hdl-ast --locked --no-fail-fast` rc=0, `Summary [24.416s] 8043 tests run: 8043 passed, 1 skipped`, wall 432 s, no FAIL/TRY/TIMEOUT/SIGSEGV/SIGABRT/LEAK-FAIL/SLOW lines.
- run2 after the parenthesization (s580/gate): `cargo fmt --all -- --check` rc=0 (3 s); `cargo clippy --workspace --all-targets --locked -- -D warnings` rc=0 (92 s); `cargo nextest run -p cli -p elaborate -p hdl-parser -p hdl-ast --locked --no-fail-fast` rc=0 (448 s) `Summary [24.682s] 8043 tests run: 8043 passed, 1 skipped`; FAIL 0, TRY-FAIL 0, TIMEOUT 0, SIGSEGV/SIGABRT/ABORT 0, LEAK-FAIL 0, SLOW 0.
- Not run (outside the brief's scoped gate): sim-engine / sim-ir / other crates' tests, `cargo test --doc`, the product-shape (no-oracle) axis, `--features jit`.

## POST freeze
- `cargo build --release -p cli --locked` rc=0 (59 s) → s580/post/vita md5=65a8ab183a63664b3b26ac4607967a2c size=7304736 (PRE 9d37b3cdbd7f325fe273241dfbb3f465 / 7304752). Tree not edited after the freeze.
- `git diff --stat`: 24 tracked files changed, 162 insertions(+), 144 deletions(-); new untracked: crates/elaborate/src/wildcard_eq.rs (326 lines), crates/cli/tests/inside_wildcard.rs (909 lines).
- Census re-run on POST release: 146 cell files byte-identical to the debug POST outputs after path normalisation (2 apparent diffs were stale debug files of fixtures edited afterwards; their current release output == debug output).

## IR byte-identity (release PRE vs release POST, `vita vcmp` + `vita velab`, s580/ident)
10 designs: I01 (= test control F14: variable/2-state/string/real/handle/fill/constraint/generate elements), I02 (N01 battery), I03 (`==?` cells the builder must leave alone: unsigned patterns, same-width signed, signed LHS + unsigned narrower pattern, 64/128-bit, fill; plus x/z-free inside), I04 (class_crv.rs inside/implication constraints), I05 (string_cast.rs `s inside {24'h610062}` + string set), I06 (sva_paren_property `a |-> b inside {1'b1}`), I10–I13 (examples/). Every `.velab` is the same size PRE vs POST and differs in exactly 32 contiguous header bytes (cmp offsets 42–73 = the recorded upstream `.vu` digest, which changes because `.vu` carries the re-pinned hdl-ast schema hash); every SimIr body byte is identical; every run output identical.

## Corpus — `cargo run -p corpus-runner --locked -- run` (target/release/vita == POST md5), rc=0, wall 363 s
```
workload           tool       grade          median  detail
sha256             vita       ok             1.197s  
aes                vita       ok             2.522s  
picorv32           vita       ok             4.231s  
darkriscv          vita       ok             3.776s  
biriscv            vita       ok             3.983s  
serv               vita       ok             7.278s  
verilog-axi        vita       ruled-split         -  ruled split — ROADMAP §2-N: t0 continuous-assign event order; iverilog answers two ways to the same question (fires for `a|b`, not for `a&b`)
verilog-ethernet   vita       ok             2.232s  
ibex               vita       ok            30.024s  
keccak             vita       ok             4.063s  
keccak-arr         vita       ok            12.415s  
coverage: 11/11 of the corpus runs under vita
```
(The cloned corpus sources hold 503 `inside {` lines — UVM/DV files — none with an x/z literal element; the corpus therefore certifies the x/z-free path only.)

## Residues recorded, not fixed (cell + oracle text)
1. Runtime x/z element (4-state variable): `v=4'b1100; e=4'b1x00; v inside {e}` → vita x (PRE and POST); sv2v→iverilog `1`; iverilog `v ==? e` `1`; verilator refuses (`RHS of ==? or !=? is fourstate but not a constant`). Same for `e=4'b1z00` (v=1000), `e=4'bxxxx`, `{4'b0000, e}`. Prerequisite: a runtime `==?` (unk-plane) IR primitive — ROADMAP §3.b "deep" row.
2. Function-call element returning `4'b1?00`: vita x; sv2v→iverilog 1; verilator refuses.
3. Const tree producing x from known leaves `v inside {4'd1/4'd0}`: vita x; iverilog `v ==? (4'd1/4'd0)` 1; verilator 1.
4. Compound x/z element is now LOUD (E40 E41 E43 E44 E45 E46 E47 E82) — both oracles run them (1, E45 0). `const_wide::fold_self_bits` (carry-free 4-state walk) is the likely way to fold these into one pattern for both `inside` and `==?` (§3.b compound-==?).
5. Const domain: two x/z elements `localparam L = PV inside {4'b0?00, 4'b1?00}` loud (sv2v→iverilog 1) — same as PRE `(PV ==? 4'b0?00) || (PV ==? 4'b1?00)`, which is loud too. A signed x/z pattern in a constant context loud (`S=8'sd84; S inside {4'sb?100}`: sv2v→iv 1, verilator 0).
6. Pre-existing const-domain `==?` operator with a signed pattern (unchanged, InsideEq not routed there): `localparam signed [7:0] S = 8'sd84; L = S ==? 4'sb?100` vita 0 / iverilog 1 / sv2v→iv 1 / verilator 0; `S2 = 8'sd12; L2 = S2 ==? 4'sb1?00` vita 1 / iverilog 0 / verilator 0. Silent-wrong, `const_compare_special` masked compare zero-extends a signed pattern.
7. Constraints with an x/z element stay E3009 (verilator 5.052 returns randomize()=0 — no oracle).
8. Constant-function body with an x/z element stays E3009 (`cf(…) has no constant-fold arm`); both oracles 1.
9. Out of scope per brief: `case (e) inside` (E2002), the duplicated LHS in `parse_inside` (comment kept), unpacked-array element (E3009).
10. Possible pre-existing debug-only panic flagged by the census agent (not built/run): `expr_size_ctx.rs:728` calls `map_binop` before inspecting the op, so a `WildEq` under arithmetic inside a size cast (`8'(a + (b ==? 4'b1x0x))`) would hit `debug_assert!(false, "WildEq must be lowered via lower_wildcard_eq")` in a debug build. InsideEq maps to `Eq` without an assert.
11. Stale text noticed (not edited): `crates/hdl-ast/tests/schema_hash.rs` header says to bump the `.vu` format_version (no re-pin ever did); the schema-gate message names "sim-ir type shape" for a `.vu`.
12. Process note: target/debug/vita in this checkout now reflects the slice (another session was observed running probes with this path at 16:11).

## What a reviewer should attack first
- `wildcard_cmp_ids` sign rule: it reads the left operand's sign from `canonical_self_width` (the engine's width-table rule) and stamps BOTH constants signed only when `both_signed && lhs narrower`; check every left-operand kind whose canonical sign could differ from what the engine uses at run time (calls, class fields, hierarchical placeholders, `TwoState`, ArrayItem), and the `==?` operator cells that moved (F16).
- `inside_value_cmp`'s decline set: `expr_may_be_unknown` false ⇒ `==`. Any element the predicate calls known but which carries a don't-care bit would stay `==` silently. And the loud set: `xz_literal_reaches_value` descends into Select offsets/width and every Binary — check for working designs it now refuses (a literal x/z inside an index of a runtime element).
- The `==?` operator behaviour change (signed extension, unsized x/z extension) is outside the row's literal wording; it is the shared builder per ER §2.3 and measured against iverilog's own `==?`.
- Const domain: `fold_region` declines an x/z InsideEq element — confirm no consumer of the wide fold turns that decline into a silent default (measured consumers: localparam typed/untyped, generate if/for, range bound, instance override, `!`/`?:` in localparam — all right or loud).


# Round 2 (on top of e5147442)
Lens reports: lens_snd/REPORT.md, lens_diff/REPORT.md. Work dir: s580/r2. Re-run harness: r2/rerun_lens.py (re-runs every lens probe variant with s580/post/vita — must reproduce the lens' recorded POST text, which validates the define set — and with the new binary), r2/cmp_census.py (round-1 census + fixtures, POST vs new).

## R2 design (as built)
- R2-1 / R2-2: ONE constant-domain wildcard rule set.
  - `crate::wildcard_eq::pattern_ext_fill(msb_xz, msb_val, both_signed, unsized_xz)` — the extension rule, now read by the run-time builder AND both constant domains.
  - `crate::const_wide::wildcard_match(l, p, pw, w, fill) -> Option<bool>` — the 4-state compare (0 on a known mismatch, x on a compared x/z left bit, else 1), read by both constant domains.
  - i64 domain: `Elaborator::const_wildcard_i64` (in wildcard_eq.rs), called from `eval_const_env_at`'s comparison arm (const_fn_width.rs) right after `const_compare_special`, only with no local bindings (constant-function bodies keep PRE's decline): pattern = a non-fill literal with x/z (through parens); `w = max(const_self_width(lhs), pw)` (≤ 64), sign = both signed (`const_signed_env`), LHS = `eval_const_env_at(lhs, …, w, sign)` — carries / shifts / `~` at w. Anything else → None → the existing `ast_holds_unknown_literal` → `selfdet_bits_i64` (wide domain).
  - wide domain: `fold_region`'s comparison arm admits `WildEq`/`WildNe`/`InsideEq`; a pattern with an x/z bit must be a literal (incl. fill, through parens) or the arm declines (as the run time refuses / does not fold a compound x/z pattern); LHS refolded at w with the region sign (`at` + `widen_to`), pattern extended by `pattern_ext_fill`; without an x/z bit the op is mapped to `Eq`/`Ne` and takes the existing path. `wide_top_is_self_determined` lists `WildEq`/`WildNe`. Round-1's `InsideEq` decline removed.
  - `const_compare_special` (const_str.rs) keeps only the string fold; its self-width masked compare (the F1 mechanism, also the pre-existing `==?` silent-wrong) is deleted.
- R2-3: `wildcard_cmp_ids` with no left width → `wildcard_cmp_or_form`: `(lhs | W) ==/!= (P|W)` at the pattern width with the pattern's sign; an unsized x/z-MSB pattern there is loud. Known-width IR unchanged (AND form). Deviation from the brief: a SIGNED pattern also takes the OR form without a known left sign — hand-spelled on PRE (r2/OR1.sv, OR2.sv; 25 absolute-path cells incl. 10 signed / mixed-sign and x/z left bits) every cell printed iverilog's own `==?` text, because the engine signs the `|`/`==` region from the resolved operand.
- R2-4: unsizedness recorded where the literal is lowered (`Elaborator::unsized_xz_lits`, expr ids of UnsizedBased literals whose MSB is x/z, set in `lower_expr_ungated`'s IntLit arm); `wildcard_cmp_ids` takes the pattern's expr id and reads the set, so a `let` body, a parenthesis or any other route delivering the same node carries the fact. Other indirections: a parameter / enum label holding x/z is refused at its declaration (E3009), a function call / array item is not a `Const` (runtime residue), a cast / concat / ternary is a compound element (loud) — the `let` body was the only one that produced the literal's own `Const` behind another AST.
- R2-5: `lower_size_ctx` maps the operator lazily (`let irop = || map_binop(*op)`), so the comparison arm that falls to `lower_size_leaf` never calls `map_binop` on `==?`.
- R2-6: after R2-1, a census of 18 x/z compare shapes × 5 constant positions (r2/r26/S*.sv: B range bound `[(E):0]` $bits, A array dimension `ad [(E):0]` $size, G generate-if, O instance override, L localparam; PRE / e5147442 / new / sv2v→iverilog / verilator). Every `inside`/`==?` shape that FOLDS now gives the oracle value in all five positions. Shapes that still DECLINE — a compound x/z pattern (`{2'b1?, 2'b00}`, `4'(4'b1?00)`) and an x-valued wildcard (x/z left bit under a compared pattern bit) — were LOUD in G/O/L and a SILENT default in B (1 bit) and A ($size x); plus a generate-`case` item (r2/r26/X4.sv), where an unfoldable label is skipped as a non-match (sv2v→iverilog takes the item; verilator refuses x/? labels). Fixed, scoped to x/z-bearing `==?`/`!=?`/InsideEq nodes via one predicate `wildcard_eq::holds_xz_wildcard`: `check_const_range_bound` (the one funnel for packed ranges, array dimensions and parameter ranges) refuses with E3009 after its existing reasons; the generate-`case` label loop refuses with E3010 (Nets phase, the scrutinee's pattern). `nonconst_bound_reason` untouched.

### R2-6 decline table (new build; PRE → e5147442 → new; oracle = sv2v→iverilog / verilator)
| shape | B range bound | A array dim | G generate-if | O override | L localparam |
|---|---|---|---|---|---|
| S01 `4'b1100 inside {4'b1?00}` | 1→2→2 (2/2) | x→2→2 | E3010→then→then | E3009→1→1 | E3009→1→1 |
| S02 `… {4'sb1?00}` | 1→1→**2** (2/2) | x→x→**2** | E3010→E3010→then | E3009→E3009→1 | E3009→E3009→1 |
| S03 `… {'b1?00}` | 1→1→**2** | x→x→**2** | E3010→E3010→then | →1 | →1 |
| S04 `… {4'b0000, 4'b1?00}` | 1→1→**2** | x→x→**2** | →then | →1 | →1 |
| S05 `… {{2'b1?, 2'b00}}` (compound) | 1→1→**E3009** (2/2) | x→x→**E3009** | E3010 (unchanged) | E3009 | E3009 |
| S06 `4'b1100 ==? 4'b1?00` | 2/2/2 | 2 | then | 1 | 1 |
| S07 `4'b1100 ==? {2'b1?, 2'b00}` (compound) | 1→1→**E3009** (2/2) | x→x→**E3009** | E3010 | E3009 | E3009 |
| S08 `4'bx100 inside {4'b1?00}` (value x) | 1→1→**E3009** (iv "x", vl refuses) | x→x→**E3009** | E3010 | E3009 | E3009 |
| S09 `4'b1100 == 4'b1?00` (plain `==`, x) | 1 (iv 1) — unchanged, out of scope | x (iv 1) — unchanged | E3010 | E3009 | E3009 |
| S10 `4'b1100 !=? 4'b1?00` | 1 (=) | 1 | else | 0 | 0 |
| S11 `(4'd15+4'd1) inside {5'b1?000}` | 1→1→**2** | x→1→**2** | E3010→else→**then** | E3009→0→**1** | E3009→0→**1** |
| S12 `4'b0100 inside {4'b1?00}` | 1 | 1 | else | 0 | 0 |
| S13 `4'b1100 inside {'x}` | 1→1→**2** (vl 2; sv2v crashes) | x→x→**2** | →then | →1 | →1 |
| S14 `4'bx100 ==? 4'b1?00` (value x) | 1→1→**E3009** (iv 1, vl refuses) | x→x→**E3009** | E3010 | E3009 | E3009 |
| S15 `… {(4'b1?00)}` | 1→1→**2** | x→x→**2** | →then | →1 | →1 |
| S16 `… {4'(4'b1?00)}` | E3009 (all) | E3009 | E3010 | E3009 | E3009 |
| S17 `4'b1100 ==? 4'(4'b1?00)` | E3009 (all) | E3009 | E3010 | E3009 | E3009 |
| S18 `4'b1x00 > 4'd2` (relational, x) | 1 (iv 1, vl 2) — unchanged, out of scope | x — unchanged | E3010 | E3009 | E3009 |
| generate-`case` item `(4'b1100 inside {{2'b1?, 2'b00}})` (X4) | | | default→default→**E3010** (sv2v→iv item, vl refuses) | | |
Still a silent default after round 2 (recorded, not inside/`==?`-specific, PRE-identical): S09 / S18 in B and A (plain `==` / relational with an x literal: `$bits` 1 = iverilog's 1, `$size` x vs iverilog 1); an x-valued wildcard inside a `?:` that sizes a replication count or a part-select (`{(x?2:3){1'b1}}` 0000, `v[(x?1:2):0]` 1 — the same with plain `==`, r2/r26/X2.sv).

## R2 tests
- crates/cli/tests/inside_wildcard.rs: `constant_and_constraint_x_z_shapes_stay_loud` replaced by `shapes_without_a_constant_value_are_loud` (constraint, constant-function body, range bound compound `inside`, array dim x-valued `inside`, range bound compound `==?`, generate-case item, OR-form unsized x pattern) — its two former constant cases fold now (pinned in `constant_wildcard_eq_operator_and_sets` I1/I2). New value tests: `constant_wildcard_reads_the_left_operand_at_the_common_width` (H01), `constant_wildcard_eq_operator_and_sets` (H02), `constant_definite_mismatch_and_bounds` (H03), `absolute_hierarchical_left_operand` (H04), `let_reference_to_an_unsized_x_literal` (H05), `wildcard_eq_under_a_size_cast_debug_build` (H06). Fixtures + PRE / e5147442 / new outputs: r2/fx2/H0*.{sv,pre,post,new}; oracles r2/fx2 (sv2v→iv, verilator, iverilog direct H02q, H05w).
- Teeth (test exe with target/debug/vita swapped, fresh inode, restored + md5 verified): vs the e5147442 debug binary (s580/vita_debug_post.bak, md5 e429b343c801e1138fba2d05a9c68d28 — the one the lens recorded) 7 FAILED / 19 ok: exactly the 7 round-2 tests (H06 = the debug panic, rc 101). vs PRE 23 FAILED / 3 ok (backends_agree, the control, the residue). Raw: r2/teeth_vita_debug_post.bak.txt, r2/teeth_pre_vita.txt.
- Re-measured deliberate-loud pins the change turned into values (all oracles agree with the new value; converted per ER §7.3, prose moved):
  - crates/cli/tests/string_literal_condition.rs `a_wildcard_compare_of_a_string_literal_stays_refused` → `a_wildcard_compare_of_a_string_literal_folds`: `"a" ==? 8'b0110_000x` / `8'b0110_001x` / `"a" inside {8'b0110_000x}` in generate-ifs → `G0 1`, `G1 0`, `G2 1` (iverilog 13.0 direct, verilator 5.052, sv2v → iverilog all print these; PRE and e5147442 E3010). The refusal text that test quoted (const_bound.rs `unfoldable_reason_in`) said "no constant-fold arm for a string-literal operand", which is now false — reworded to what still fails ("its x/z pattern is not a single literal, or its result is x").
  - crates/sim-engine/tests/const_domain_semantics.rs `wildcard_eq_negative_lhs_is_loud` → `wildcard_eq_negative_lhs_folds` (`P=-6: P ==? 4'b1x1x` 1, `4'b0x1x` 0) and `wildcard_eq_unsized_pattern_is_loud` → `wildcard_eq_unsized_pattern_pads_to_the_expression` (`40'hFF_0000_0000 ==? 'hx` 1, `'h0x` 0): iverilog 13.0 and verilator 5.052 print `A=1 A2=0 M=1 M2=0` (r2/CD1.sv); PRE and e5147442 E3009. Their docstrings had already recorded iverilog's 1 as "a correct-or-loud over-rejection". (sim-engine is outside the brief's scoped gate; run separately: `cargo nextest run -p sim-engine --locked --test const_domain_semantics --no-fail-fast` rc=0, 15 passed.)

## R2 gate (s580/gate, run3 = final tree)
- `cargo fmt --all -- --check` rc=0 (2 s); `cargo clippy --workspace --all-targets --locked -- -D warnings` rc=0 (90 s); `cargo nextest run -p cli -p elaborate -p hdl-parser -p hdl-ast --locked --no-fail-fast` rc=0 (434 s) `Summary [24.505s] 8049 tests run: 8049 passed, 1 skipped`, 0 FAIL/TRY-FAIL/TIMEOUT/SIGSEGV/SIGABRT/ABORT/LEAK-FAIL/SLOW lines.
- Extra (not in the brief): `cargo nextest run --workspace --locked --no-fail-fast` rc=0 (499 s) `Summary [41.052s] 8915 tests run: 8915 passed, 15 skipped`. Not run: `cargo test --doc`, the no-oracle axis, `--features jit`.
- Earlier run2 (before the string/sim-engine pin conversions): scoped nextest rc=100, 1 FAIL `cli::string_literal_condition a_wildcard_compare_of_a_string_literal_stays_refused` (`left: Some(0) right: Some(1)`, vita printed `G0 1` = all three oracles) — re-measured and converted, see R2 tests.

## POST2 freeze
- `cargo build --release -p cli --locked` rc=0 → s580/post2/vita md5=09ab4bd248b02f4675f532e2d342282e, 7304736 bytes. `git diff --stat e5147442`: 16 files changed, 769 insertions(+), 248 deletions(-) (no untracked additions beyond .DS_Store).
- Lens re-run on POST2 (r2/rerun_lens.py; r2/lens/*.post2*.txt): 66 output files byte-identical to the debug build; snd P4b/P5/P8 (recorded with absolute paths) identical to debug. Census re-run on POST2 (r2/census_post2.txt): 164 files, 5 changed vs POST, all intended (below). R2-6 census on POST2: 94 files identical to debug.
- IR identity POST → POST2 (vcmp + velab, s580/ident): 25 designs (the 10-design battery + every round-1 test fixture F01–F16, incl. run-time x/z elements and the F12 constant contexts): `.vu` and `.velab` byte-identical. Round 2 changed no IR of a known-width left operand.

## Corpus on POST2 — rc=0, wall 362 s
```
sha256 ok 1.192s · aes ok 2.511s · picorv32 ok 4.233s · darkriscv ok 3.767s · biriscv ok 3.970s · serv ok 7.315s
verilog-axi ruled-split (ROADMAP §2-N t0 continuous-assign event order) · verilog-ethernet ok 2.228s
ibex ok 29.598s · keccak ok 4.040s · keccak-arr ok 12.385s · coverage: 11/11
```

## Intended changes POST → POST2 (every changed cell; all = oracle)
- census: Z04 / F17a `S=8'sd84; L = S inside {4'sb?100}` E3009 → 1 (sv2v→iv 1; verilator 0 = x sign bit as 0); Z05 `S ==? 4'sb?100`, `S2 ==? 4'sb1?00` `0 1` → `1 0` (iverilog direct `1 0`); F17b `PV inside {4'b0?00, 4'b1?00}` E3009 → 1 (sv2v→iv 1); G1 `(PV ==? 4'b0?00) || (PV ==? 4'b1?00)`, `!(…)`, `?:` E3009 → `1 0 5` (iverilog `1 0 5`).
- lens_snd: P1 L1 L3 L5 L7 0→1, `==?` L2 L4 L6 0→1, bits(rb) 1→2, G1 else→then; P2a L2 L4 0→1, bits(rbq) 1→2, GQ else→then; P2b bits(rb) 1→2; P2c LB 0→1, override P 0→1; P4a rbs rbu rb2 1→2, rbuq rb2q 1→2; P5 E3009 → `LW=0 LN=0 F1..F3=1 / LWQ=0 FQ=1` (= verilator); P7 L4 1→0, L4q 1→0, G4 then→else; P8 E3009/E3010 → `L1=0 L3=0 LW2=0`, `G1 else`. All = sv2v→iverilog / iverilog direct / verilator as recorded by the lens.
- lens_diff (inside and -DIV `==?` variants): P03 L09 L10 L12 L14 0→1, G09 else→then; P03 -DB E3009 → every value = iverilog; P05 -DK5 and P07 -DLETT K5b 0→1 (let); P07 L20 L21 L22 1→0, AB 8→4, G20 then→else; P08 E3009 → H01–H05, W01–W08, Q01, Q02 values = iverilog; P08b L01q 0→1, L02q 1→0, L09q 0→1, L20q 1→0; P09 / P09 -DNOP E3009 → A1 0, A2 1, P1 0, P2 1, P3 1, P4 1 (= iverilog); P10b L30 L31 0→1, OV 8→2. Only remaining lens_diff mismatch vs iverilog: P07 K1a/K1c/PV1 (F2, recorded).

## R2 findings table (PRE 00c3d76d / POST e5147442 / POST2 / oracle)
| id | status | cells | mechanism changed |
|---|---|---|---|
| R2-1 (snd F1, diff F1) BLOCKING | fixed | snd P7 L4 `(4'd15+4'd1) inside {5'b0?000}` 0 / 1 / 0 / 0 (vl; iv `==?` 0), G4 else/then/else/else · diff L20 L21 L22 0/1/0/0, AB 4/8/4/4, G20 else/then/else/else, OV W=2/8/2/2 · snd P1 L1 L3 L5 L7 E3009/0/1/1, G1 E3010/else/then/then, bits(rb) –/1/2/2 · diff L09–L14 E3009/0/1/1, G09 E3010/else/then/then · L30 L31 E3009/0/1/1 · snd P2c LB, override E3009/0/1/1 · `==?` operator (pre-existing): P2a L2 L4 0/0/1/1, P7 L4q 1/1/0/0, P08b L01q 0/0/1/1 L02q 1/1/0/0 L09q 0/0/1/1 L20q 1/1/0/0, Z05 `0 1`/`0 1`/`1 0`/`1 0` | `const_compare_special`'s self-width masked compare deleted; `const_wildcard_i64` (i64, LHS at w) + `fold_region` wildcard arm (wide) share `pattern_ext_fill` and `wildcard_match` |
| R2-2 (snd F4) | fixed | P8 `4'b0100 inside {4'sb1?00}` 0/E3009/0/0, `{'b1?00}` 0/E3009/0/0, 100-bit `W inside {4'b000?}` 0/E3009/0/0, generate-if else/E3010/else/else | round-1 `fold_region` InsideEq decline removed; the wide arm answers a definite mismatch |
| R2-3 (diff F5) | fixed | P09 A1 0/E3009/0/0 (iv), A2 x/E3009/1/1, P1 0/E3009/0/0, P2 x/E3009/1/1; P08 H01–H05 → iverilog values; `==?` on an absolute path E3009/E3009/value; H04 signed / x cells = iverilog | `wildcard_cmp_or_form` `(lhs \| W) ==/!= (P\|W)` when `ir_bits_of(lhs)` is None; unsized x/z-MSB pattern there loud |
| R2-4 (diff F3) | fixed | `let LU = 'bx1; v36 inside {LU}` 0/0/1/1 (direct `'bx1` –/1/1, iverilog `v36 ==? 'bx1` 1), `v36 ==? LU` 0/0/1/1 | unsizedness recorded at the literal's lowering (`unsized_xz_lits`), read by expr id |
| R2-5 (N1) | fixed | `8'(a + (b ==? 4'b1x0x))` debug: e5147442 panic rc 101 → `00000010`; release unchanged (= iverilog/verilator) | `lower_size_ctx` maps the op lazily |
| R2-6 (snd F3) | fixed (loud) for `inside`/`==?`; recorded for `==`/relational | range bound / array dim on a compound x/z pattern or an x-valued wildcard: 1-bit / `$size x` → E3009; generate-case item: default → E3010 (table above) | `holds_xz_wildcard` in `check_const_range_bound` and the generate-case label loop |
| F2 (snd F2 / diff F2) | recorded | `sub #(.P(4'b1x00))`: P prints 1000 PRE/POST/POST2, iverilog & verilator 1x00; `v inside {P}` 0, iverilog `v ==? P` 1; `-G TP=4'b1x00` → 1000 (iverilog -P rejects the x digit) | — (same class as §3.b xz-fill-param) |
| diff F4 | recorded | `localparam logic [3:0] CAX [0:1] = '{4'b1x00, 4'b0000}; v inside {CAX[0]}` x/x/x, sv2v→iverilog 1 | — (ArrayItem is not a Const: runtime-residue class) |
| E9002 text | recorded | a stale `.vu` reads "sim-ir type shape changed … rerun `velab`" | — |

## R2 residues (recorded, not fixed)
1. F2, F4, E9002 above.
2. Still a silent default, not `inside`/`==?`-specific (PRE-identical): a plain `==` / relational with an x literal in a range bound (`[(4'b1100 == 4'b1?00):0]` $bits 1, iverilog 1) or array dimension (`$size` x, iverilog 1); an x-valued compare inside a `?:` that sizes a replication count or part-select (`{(x?2:3){1'b1}}` → 0000, `v[(x?1:2):0]` → 1), the same with plain `==`.
3. Round-1 residues unchanged: run-time x/z element (variable / call / `4'd1/4'd0`), compound x/z element loud at run time, constant-function body loud, constraint x/z element loud. Round-1 residues 5 (two x/z elements in a constant) and 6 (constant `==?` with a signed pattern) are CLOSED.

## R2 delta hunk list (for round-2 reviewers)
- crates/elaborate/src/wildcard_eq.rs: `lower_wildcard_eq` (passes the pattern's expr id); `inside_value_cmp` (signature: operands only); NEW `const_wildcard_i64`; `wildcard_cmp_ids` (pat_id, unsized from `unsized_xz_lits`, `pattern_ext_fill`, OR-form dispatch); NEW `wildcard_cmp_or_form`, `push_known_const`, `pattern_ext_fill`, `holds_xz_wildcard`, `word_bit`; `is_unsized_literal` deleted.
- crates/elaborate/src/const_wide.rs: NEW `wildcard_match`; `fold_region` comparison arm (WildEq/WildNe/InsideEq wildcard path, op mapping to Eq/Ne, round-1 decline removed); `wide_eq_with_unknowns` and the `ord` arm (round-1 InsideEq additions reverted); `wide_top_is_self_determined` (+WildEq, WildNe).
- crates/elaborate/src/const_fn_width.rs: `eval_const_env_at` comparison arm (calls `const_wildcard_i64`).
- crates/elaborate/src/const_str.rs: `const_compare_special` (wildcard masked compare deleted, doc); `unsigned_sized_xz_literal` deleted.
- crates/elaborate/src/const_eval.rs: `check_const_range_bound` (x/z wildcard refusal, `return` after the call refusal).
- crates/elaborate/src/generate.rs: the `GenItem::Case` label loop (x/z wildcard label refusal).
- crates/elaborate/src/expr_size_ctx.rs: `lower_size_ctx` (lazy `map_binop`).
- crates/elaborate/src/expr_main.rs: `lower_expr_ungated` IntLit arm (records `unsized_xz_lits`); `Binary` arm (new `inside_value_cmp` call).
- crates/elaborate/src/expr_ctx.rs: `lower_expr_ctx` (new `inside_value_cmp` call).
- crates/elaborate/src/lib.rs + driver.rs: `Elaborator::unsized_xz_lits` field + init.
- crates/elaborate/src/const_bound.rs: `unfoldable_reason_in` (comment, refusal wording); crates/elaborate/src/const_fn.rs: comment.
- tests: crates/cli/tests/inside_wildcard.rs, crates/cli/tests/string_literal_condition.rs, crates/sim-engine/tests/const_domain_semantics.rs.


# Round 3 (on top of cf5076ef, last review round)
Lens reports: lens_diff2/REPORT.md (Q1–Q9), lens_snd2/REPORT.md (P1–P10). Work dir s580/r3; fixtures r2/fx2/H07.sv, H08.sv; lens comparison harness r2/cmp_all_lens.py (every .sv in the four lens dirs × no define, each single `ifdef/ifndef/elsif` define, and each with -DIV where the file names IV; BASE vs NEW); census r2/cmp_census2.py (BASE vs NEW over the round-1 census, fixtures, r2/fx2, r2/r26).

## R3 changes (minimal)
- R3-1 (wildcard_eq.rs `wildcard_cmp_ids`): `both_signed` is asked whenever the pattern is signed (was: only when `aw != pw`), from `canonical_self_width(lhs)` — None stays loud, message now "with a signed pattern needs the left operand's signedness"; mask and cleaned constants are typed `both_signed` (was `both_signed && aw < w`). An unsigned pattern or unsigned left operand keeps unsigned constants → that IR is unchanged. Doc rewritten.
- R3-2 (wildcard_eq.rs): `wildcard_cmp_or_form` deleted; `ir_bits_of(lhs) == None` returns to round 1's loud E3009 "on a left operand of unsizable width is unsupported (the pattern mask must cover it)" (now naming `inside` too) for `==?` and `inside`. The `unsized_xz_lits` record (R2-4) stays.
- R3-3 (generate.rs, `GenItem::Case` label loop): refuse only when `lv` is None, the label holds an x/z wildcard AND `const_wide::fold_self_bits(lab, wide_name_bits)` declines; an x-valued wildcard label is skipped as a non-match. Message: "… whose x/z pattern is not a single literal has no constant value". Range-bound / array-dimension refusals unchanged.

## R3 tests (crates/cli/tests/inside_wildcard.rs, 28 tests)
- `absolute_hierarchical_left_operand` (R2-3 value pin) → `left_operand_without_a_width_is_loud`: `t.uL.u8 inside {4'b?100}`, `t.uL.u8 ==? 4'b?100`, `t.uL.v36 ==? 'x`, `t.uL.r ==? 4'b1?00`, `string s; s ==? 8'b0110_000x` → rc 1, E3009 "left operand of unsizable width", no value printed. `shapes_without_a_constant_value_are_loud`: the absolute-path unsized case's needle → "left operand of unsizable width".
- NEW `signed_comparison_signs_the_left_operand_operators` (r2/fx2/H07.sv): run-time `R`/`more` and constant `C` twins, `inside`, `==?`, `!=?`, an unsigned-pattern control; = sv2v→iverilog = verilator on every cell.
- NEW `x_valued_generate_case_label_is_a_non_match` (r2/fx2/H08.sv): `XB one`, `XI one`, `XD default`, `FOLD item` = sv2v→iverilog (verilator refuses x/? generate-case labels).
- Teeth (test exe, target/debug/vita swapped, fresh inode, restored + md5 verified): vs POST2 = cf5076ef release (s580/post2/vita): 4 FAILED = exactly the round-3 tests (`left_operand_without_a_width_is_loud`, `shapes_without_a_constant_value_are_loud`, `signed_comparison_signs_the_left_operand_operators`, `x_valued_generate_case_label_is_a_non_match`); vs PRE: 25 FAILED, 3 ok (backends_agree, control, residue). Raw: r3/teeth_post2_vita.txt, r3/teeth_pre_vita.txt.

## R3 gate (s580/gate, final tree)
- `cargo fmt --all -- --check` rc=0 (2 s); `cargo clippy --workspace --all-targets --locked -- -D warnings` rc=0 (94 s); `cargo nextest run -p cli -p elaborate -p hdl-parser -p hdl-ast --locked --no-fail-fast` rc=0 (434 s) `Summary [24.496s] 8051 tests run: 8051 passed, 1 skipped`, 0 FAIL/TRY-FAIL/TIMEOUT/SIGSEGV/SIGABRT/ABORT/LEAK-FAIL/SLOW.
- Extra: `cargo nextest run --workspace --locked --no-fail-fast` rc=0 (492 s) `Summary [41.184s] 8917 tests run: 8917 passed, 15 skipped`. Not run: `cargo test --doc`, no-oracle axis, `--features jit`.

## POST3 freeze
- s580/post3/vita md5=76ad4738a0ec5a26cc7718c675467a9e, 7304736 bytes. `git diff --stat cf5076ef`: 3 files changed, 182 insertions(+), 150 deletions(-) (crates/cli/tests/inside_wildcard.rs, crates/elaborate/src/generate.rs, crates/elaborate/src/wildcard_eq.rs).

## R3 re-runs (POST2 → POST3)
- Four lens probe sets (r2/lens_r3_post3.txt; 201 variants): 47 changed, identical to the debug-build run, every one intended:
  - R3-1: lens_diff2 Q1 R41 R42 0→1; Q2 R41–R44 R46 R47 R49 R52 0→1, R48 1→0 (inside, `==?`, with and without -DCONSTS — the C twins unchanged at 1/0); Q7 Z1 Z3 Z4 0→1; lens_snd2 P2 RI `0 0 0 0 0`→`1 1 1 1 1`, RQ `0 0 0 0`→`1 1 1 1` (KI/KQ unchanged 1). All = iverilog / sv2v→iverilog / verilator.
  - R3-2 (now E3009 "left operand of unsizable width", round-1 behaviour): lens_diff P08 (H01–H05 and the W/Q cells of that file), P09 (A1 A2 P1 P2 P3 P4, also -DNOP, -DIV); lens_diff2 Q4 (H01–H37, -DSTM), Q8 (K1–K6 `t.u.i32` cells, -DIU, -DRR); lens_snd2 P1 (fill on absolute paths, -DNO_INSIDE, -DONLYC), P3 (absolute-path cells, -DNO_INSIDE), P7 (+ -DNO_INSIDE, -DU), P8 -DREALI / -DREALQ, P9 -DSFMT / -DSTRV.
  - R3-3: lens_diff2 Q3 -DXBEFORE E3010 → `G1 then … GC1 one …` (= iverilog), Q6 -DXB / -DXI E3010 → `XB one` / `XI one` (= iverilog); lens_snd2 P6 -DXV / -DXI E3010 → `G1 default` (= iverilog direct and sv2v→iverilog).
- Census + fixtures (r2/census_r3_post3.txt; 266 files): 6 changed, all intended — r2/fx2/H04 (R2-3 fixture) → E3009; r2/fx2/H07 (`R 0 0 0 0 0 0 0 1 1`→`R 1 1 1 1 1 1 1 0 1`, `more 0 0 1 0`→`1 1 1 1`); r2/fx2/H08 E3010 → values; r2/r26/X1, X4 (still E3010, message text only); r2/r26/X2 E3010 → `X2 rep 0000 / psel 1 / gencase default` (the gencase line = R3-3; rep/psel = recorded G3 residue, PRE-identical).
- IR identity POST2 → POST3 (vcmp+velab): battery + fixtures (25 designs): byte-identical except I03, F04, F16 (they hold both-signed `==?`/`inside` cells). Per-cell (r3/irc, 234 single-cell designs from census A/W/S/X + `_wq` twins, F04 and H07 cells; 6 F04 cells did not compile in this harness — decl extraction missed `integer i32` — their S-group twins are measured): `.velab` changed for exactly the both-signed, left-width ≥ pattern-width cells: S01 S02 S03 S05 S12 S14 S15 S16 S17 S18 S22 (`inside`), S01q S02q S03q S05q S08q S12q S14q S15q S16q S17q (`==?`), F04 call_lhs / ss_narrow_pat_{ext_ones,msb1,msbq,msbq_pos,msbx,msbz}, H07_1–8, 10, 11, 13 — values unchanged on every census/F04 cell (no sign-sensitive operator inside), changed to the oracle's on H07 (0→1; H07_8 `!=?` 1→0). Every unsigned-pattern or unsigned-LHS cell (A, W, X groups; S04 S06 S10 S11 S13 S19 S20; H07_9, H07_12) is byte-identical.

## Corpus on POST3 — rc=0, wall 364 s
```
sha256 ok 1.179s · aes ok 2.534s · picorv32 ok 4.257s · darkriscv ok 3.772s · biriscv ok 3.984s · serv ok 7.333s
verilog-axi ruled-split (ROADMAP §2-N t0 continuous-assign event order) · verilog-ethernet ok 2.224s
ibex ok 29.943s · keccak ok 4.058s · keccak-arr ok 12.370s · coverage: 11/11
```

## R3 findings table (PRE 00c3d76d / POST e5147442 / POST2 cf5076ef / POST3 / oracle)
| id | status | cells | mechanism |
|---|---|---|---|
| R3-1 (diff G1 = snd B) BLOCKING | fixed | `(s4+s8) ==? 4'sb1?00` 1/0/0/1/1 · `(s4+s68) ==? 4'sb1?00` 1/0/0/1/1 · `(sa>>>1) inside {4'sb111?}` x/0/0/1/1 · `(s5>>>1) inside {4'sb111?}` x/0/0/1/1 (`==?` 1/0/0/1/1) · Q2 R41–R49 POST2 0 (R48 1) → POST3 = iverilog 1 (R48 0) · Q7 Z1 Z3 Z4 0→1 · P2 RI/RQ all 0→1 · constant twins C41–C49 / KI / KQ 1 throughout POST2–POST3 | `wildcard_cmp_ids`: sign asked for every signed pattern; mask/clean typed `both_signed` at any widths |
| R3-2 (snd A, snd D) BLOCKING | reverted → loud | fill `t.uL.v36 ==? 'x` E3009/E3009/0/E3009 (iverilog 1) · real `t.uL.r ==? 4'b1?00` E3009/E3009/`RQ 0`/E3009 (iverilog refuses) · string `s ==? 8'b0110_000x` E3009/E3009/1/E3009 (iverilog refuses) · lens_diff P09 A1 0/E3009/0/E3009 (iv 0), A2 x/E3009/1/E3009 (iv 1) · snd2 P1/P7/P8/P9 → E3009 | `wildcard_cmp_or_form` deleted; unsizable left operand loud again |
| R3-3 (diff G2 = snd C) | fixed | Q6 `XB` one/one/E3010/one/one, `XI` one/one/E3010/one/one · snd2 P6 XV / XI default/default/E3010/default/default · FOLD label default/item/item/item/item · Q3 -DXBEFORE → values (= iverilog) · compound label (X4) E3010 kept | generate-case label refused only when the 4-state fold declines |

## R3 residues (recorded, not fixed)
1. R2-3 shape: `==?` / an `inside` element with x/z bits on a left operand that has no width at lowering is E3009 — absolute hierarchical paths (lens_diff P09 A1: PRE 0, iverilog 0; A2: PRE x, iverilog 1; P1/P2 the `$display` twins), lens_snd2 P1 (fill: `t.uL.v36 ==? 'x` iverilog 1), P7 (cont-assign, `t.uL.f36()`, generate paths `t.uL.g[0].gv` / relative unrecorded `uL.g[0].gv`: iverilog 1 each), P8 (`real` absolute path: iverilog refuses), P9 (`string` / `$sformatf`: iverilog refuses). Prerequisite: the resolved WIDTH and KIND of a hierarchical operand at lowering.
2. diff G3 (PRE-identical, not wildcard-specific): an x-valued compare as a replication count `{((4'bx100 ==? 4'b1?00)+1){1'b1}}` → `0` (iverilog: "Concatenation repeat may not be undefined", rc 1) and as a part-select bound `v8[(4'bx100 ==? 4'b1?00)*3+1:0]` → `1` (iverilog `x`); plain `==` the same.
3. snd A tail (PRE-identical, plain `==`): `t.uL.w36 == '1` and `t.uL.v8 == '1` print 0 on PRE and POST2/POST3, iverilog 1 — a fill against a left operand with no width defaults to 32 bits (`sibling_ctx` → `ir_bits_of(..).unwrap_or(32)`).
4. diff2 M02: verilator sizes every `inside` element to the maximum, iverilog and sv2v→iverilog per element — no oracle.
5. Round-2 residues F2 (override / `-G` x/z dropped), diff F4 (`CAX[0]` ArrayItem element `==`), E9002 text — unchanged.

## R3 delta hunks (vs cf5076ef)
- crates/elaborate/src/wildcard_eq.rs: `wildcard_cmp_ids` (unsizable → loud again; `both_signed` for every signed pattern; constants typed `both_signed`; doc); `wildcard_cmp_or_form` deleted.
- crates/elaborate/src/generate.rs: `GenItem::Case` label loop (fold-decline condition, message).
- crates/cli/tests/inside_wildcard.rs: `left_operand_without_a_width_is_loud` (replaces `absolute_hierarchical_left_operand`), `shapes_without_a_constant_value_are_loud` (needle), NEW `signed_comparison_signs_the_left_operand_operators`, NEW `x_valued_generate_case_label_is_a_non_match`.


# Round 4 — revert (on top of e6c9cc8d)
Round 3 found two BLOCKING findings on the constant axis (lens_snd3 R3-A, lens_diff3 F1/F2) with the round budget spent (ER §3.6): ship the run-time half, constant domain back to PRE semantics.

## R4 changes (files vs main 00c3d76d)
- Reverted to main (byte-identical, `git diff 00c3d76d -- <file>` empty): crates/elaborate/src/generate.rs, crates/cli/tests/string_literal_condition.rs, crates/sim-engine/tests/const_domain_semantics.rs. (Restored from 00c3d76d with `git checkout 00c3d76d -- <file>` then `git restore --staged <file>` so the index is untouched.)
- Rebuilt as main + round-1 SAME-AS-EQ arms only (InsideEq treated exactly as `Eq`; `WildEq`/`WildNe` = main):
  - const_str.rs: `const_compare_special` string arm lists InsideEq (+ `want_eq`); NO `inside_wild`, the `==?` masked compare = main.
  - const_wide.rs: `wide_eq_with_unknowns` (InsideEq beside Eq, sense list), `fold_region` comparison list + `ord` arm (InsideEq beside Eq — no x/z decline, no wildcard arm), `wide_top_is_self_determined` (+InsideEq only; WildEq/WildNe removed); `wildcard_match` deleted.
  - const_eval.rs: `delay_units_in_scope` `B::Eq | B::CaseEq | B::InsideEq` only (round-2 `check_const_range_bound` refusal and `return` removed).
  - const_bound.rs: `bin_op_text` `B::InsideEq => "inside"` only (round-2 comment/wording reverted).
  - const_fn.rs: `const_binop` InsideEq beside Eq only (round-2 comment reverted). const_fn_width.rs: `binop_result_is_context_determined` InsideEq only (`const_wildcard_i64` call removed).
- wildcard_eq.rs: `const_wildcard_i64` and `holds_xz_wildcard` deleted; `pattern_ext_fill` and `word_bit` kept (read by the run-time builder `wildcard_cmp_ids`); module doc and `pattern_ext_fill` doc state that the constant half is not built.
- Kept (run-time, read by lowering): R2-4 `unsized_xz_lits` (lib.rs field, driver.rs init, expr_main.rs `lower_expr_ungated` IntLit arm record, `wildcard_cmp_ids` read), R2-5 lazy `map_binop` (expr_size_ctx.rs), R3-1 sign (wildcard_cmp_ids), R3-2 loud no-width (wildcard_cmp_ids), the `inside_value_cmp(lhs, el)` signature (expr_main.rs / expr_ctx.rs callers).
- crates/cli/tests/inside_wildcard.rs (27 tests): constant-context tests now pin PRE: `constant_contexts_keep_pre_refusal` (four designs, E3009/E3010 "has no constant-fold arm", oracle text beside as a REFUSED note), `constant_context_residue_keeps_pre_values` (PRE's silent-wrong values pinned with the oracle text: `==?` self-width/signed, range bounds / array dims, generate-case labels), `generate_case_labels_keep_pre_semantics` (x-valued labels right on PRE; FOLD residue); `signed_comparison_signs_the_left_operand_operators` is run-time only (its constant twins moved to the refusal test); `shapes_without_a_constant_value_are_loud` keeps the constraint, constant-function and absolute-path cases. Removed: the round-2 constant value tests (H01/H02/H03) and the R3-3 test. Every run-time pin kept.

## Constant half — the queue row (facts for the docs)
**Shipped (run-time half):** a value element of an `inside` set whose LOWERED form is one `Const` with x/z bits is compared with `==?` (§11.4.13) at run time, sized and signed per §11.4.5/§11.8.1 (R3-1: signed iff both operands are signed, the sign pushed into the left operand's operators), an unsized x/z-MSB literal padding to the expression (§5.7.1, recorded at lowering so a `let` carries it — R2-4); the `==?` operator shares the builder (signed / unsized extension fixed for it too); a compound x/z element is loud (E3009); a left operand with no width at lowering is loud (E3009, R3-2). Constant contexts compare an `inside` element with `==` exactly as before the slice.

**Prerequisite for the constant half:** generate-`case` labels compared in the 4-state domain at full width. Today the label loop (generate.rs `GenItem::Case`) folds each label with `const_eval_in_scope` (i64) and SKIPS a label it cannot read as a non-match — right for an x-valued label (`===` against a known scrutinee), wrong for a label whose value is known but needs the wide / 4-state fold. Measured cells (all PRE-identical on this build):
- LPA (s580/../f2/w65.sv): `localparam P1 = (4'b1100 ==? 4'b1?00); localparam [64:0] LPA = {64'd0, P1}; case (1) LPA: …` → vita `A dflt`, iverilog `A item` (pre-existing consumer defect; PRE).
- R3-A (lens_snd3 p5/p6, scrutinee 1 unless noted): H8 `$isunknown(4'bx100 ==? 4'b1?00)`, J4 `(4'bx100 ==? 4'b1?00) >> 1` (scrutinee 0), J7 `$isunknown(4'bx100 inside {4'b1?00})`, J8 `2'd2, $isunknown(4'bx100 ==? 4'b1?00)` → vita `default`, iverilog / sv2v→iverilog `item` (PRE default too; POST2 was E3010).
- lens_diff3 F1 (G1/G3): `case (1) {64'd0, (4'b1100 ==? 4'b1?00)}` (G1 L1) → vita `dflt`, iverilog / sv2v `item`; G3 M1 (`inside`), M2 (`!=?`), M4 (`65'(…)` cast), M6 (value 0, `case (0)`), M7 (101 bits) → `dflt`, oracle `item`; M10 (wide label then `1:`) → `one`, oracle `item`; plain twins L1P / M4P / M9P `dflt` (oracle `item`).
- lens_diff3 F2 M9: `localparam [64:0] LP = {64'd0, (4'b1100 ==? 4'b1?00)}; case (1) LP: …` → PRE E3009 (the constant is refused); with the cf5076ef constant fold it bound and the label was then skipped (`dflt`, oracle `item`) — opening the fold removed the loud over the skip, which is why the constant half waits.
- X4 (r2/r26/X4.sv): `case (1'b1) (4'b1100 inside {{2'b1?, 2'b00}}): …` → `default` (sv2v→iverilog `item`, verilator refuses x/? labels).
- C (lens_snd2 P6 XV/XI, lens_diff2 Q6 XB/XI, Q3 -DXBEFORE): x-VALUED labels `(4'bx100 ==? 4'b1?00)` / `(4'bx100 inside {4'b1?00})` → non-match, `G1 default` / `XB one` / `XI one` = iverilog — right on PRE and now; a fix to the label loop must keep these.

**What cf5076ef built for the constant half (start the next attempt from it):** `Elaborator::const_wildcard_i64` (i64: left operand at `w = max(L(lhs), L(pat))` via `eval_const_env_at`, pair sign `const_signed_env(lhs) && pat.signed`), the `fold_region` wildcard arm (wide 4-state: `at`/`widen_to` refold, literal-only pattern, `WildEq`/`WildNe` mapped to `Eq`/`Ne` without x/z), `const_wide::wildcard_match` (0 on a known mismatch, x on a compared x/z left bit, else 1), the shared extension rule `wildcard_eq::pattern_ext_fill` (kept: the run-time builder reads it), `const_compare_special`'s self-width masked compare deleted, and the R2-6 refusal `holds_xz_wildcard` in `check_const_range_bound` / the generate-case label loop. Cells it fixed (all = iverilog / sv2v / verilator): lens_snd P2a (`(4'd15+4'd1) ==? 5'b1?000` 0→1, `bits(rbq)` 1→2, `GQ` else→then), the P7 twin (`L4q` 1→0), census Z05 (`S ==? 4'sb?100`, `S2 ==? 4'sb1?00`: `0 1`→`1 0`), C10–C13 (E3009/E3010 → 1/1/1/then), C28 (range bound `$bits` 1→2), C29 (override E3009 → 1), C31 (generate-for E3010 → in/in), and the R2-6 range-bound / array-dimension loudness for compound / x-valued wildcards (S05/S07/S08/S14 B and A: silent 1 bit / `$size` x → E3009). Its BLOCKING defects (do not repeat): the R3-3 label narrowing skipped a known 4-state value the i64 path could not read (R3-A, diff3 F1), and the wide fold made a constant bind that a skipping label then swallowed (diff3 F2).

**Constant-domain residues that stay (PRE behaviour):**
- `==?` operator in a constant reads the left operand at its OWN width and zero-extends a signed pattern (`const_compare_special` masked compare): `(4'd15+4'd1) ==? 5'b1?000` 0 (oracle 1), `(~4'b0000) ==? 5'b1111?` 0 (1), `(4'd15+4'd1) ==? 5'b0?000` 1 (0), `S=8'sd84 ==? 4'sb?100` 0 (1), `S2=8'sd12 ==? 4'sb1?00` 1 (0), `(4'd15+4'd1) !=? 8'b0001_?000` 1 (0) — pinned in `constant_context_residue_keeps_pre_values`.
- An `inside` element with x/z bits in a constant is refused (E3009 localparam / override, E3010 generate-if; oracle values in `constant_contexts_keep_pre_refusal`).
- C28-class range bound / array dimension: a declined x/z compare becomes a 1-bit net / `$size` x silently (`[(4'b1100 inside {4'b1?00}):0]` `$bits` 1, oracle 2; `[(4'b1100 ==? {2'b1?,2'b00}):0]` 1, oracle 2; `ad[(4'bx100 inside {4'b1?00}):0]` `$size` x).
- Generate-case label skips (above).
- A constant-function body with an x/z pattern: E3009 (both oracles 1).

**Run-time residues:**
- No-width left operand (R3-2 / diff F5): `==?` / x/z `inside` on an absolute hierarchical path, a `string`, a string sysfunc, an unrecorded generate path → E3009 (lens_diff P09 A1: PRE 0 = iverilog 0; A2: PRE x, iverilog 1; snd2 P1 fill `t.uL.v36 ==? 'x` iverilog 1; P7 generate paths iverilog 1; P8 `real` and P9 `string`: iverilog refuses). Prerequisite: the resolved width AND kind of a hierarchical operand at lowering.
- diff3 F3: a SIGNED pattern whose left operand holds an absolute-path-bearing sub-expression (index, ternary condition) is loud — `canonical_self_width` answers None for any placeholder in the subtree: `((t.uL.k != 0) ? s4 : s4) ==? 4'sb1?00` H2, `inside` twin H2I, `mem[t.uL.k] ==? 8'sb1?00_0000` H3, `s8[t.uL.k*4 +: 4] ==? 4'sb1?00` H4: PRE 1 (H2I x) = iverilog 1 → E3009; H8: PRE 0 (iverilog 1) → E3009.
- Run-time x/z element (4-state variable holding x/z, a function returning a pattern, `4'd1/4'd0`): compared with `==` → `x` (iverilog / sv2v→iverilog 1).
- diff F4: a constant unpacked-array element `CAX[0]` holding `1x00` (ArrayItem) → `==` (x; sv2v→iverilog 1).
- F2: an instance override `#(.P(4'b1x00))` and `-G TP=4'b1x00` bind `1000` (x/z lost; iverilog and verilator keep `1x00`) — same class as §3.b xz-fill-param.
- diff G3: an x-valued compare as a replication count → `0` (iverilog "Concatenation repeat may not be undefined") and as a part-select bound → `1` (iverilog `x`); plain `==` the same.
- A fill against a left operand with no width defaults to 32 bits (`t.uL.w36 == '1` 0, iverilog 1 — plain `==`).
- diff2 M02: verilator sizes all `inside` elements to the maximum, iverilog per element — no oracle.

## R4 measurement (frozen binaries: PRE 9d37b3cd…, POST3 76ad4738…, POST4 4ace7617…)
- r4/cmp3.py: every probe variant (no define, each single `ifdef/ifndef/elsif` define, each also with -DIV / -DNO_INSIDE where the file names them) of the census battery (census/, census/e|c|cq|z), the fixtures (fx/, r2/, r2/fx2, r2/r26, r4/fx), the report repro and ident battery, all six lens probe sets (lens_snd, lens_diff, lens_diff2, lens_snd2, lens_diff3, lens_snd3/p) and f2/w65.sv — 666 variants, byte compare after stripping W1017 and the path prefix:
  - both (POST4 = PRE = POST3): 269
  - = PRE only: 200 — every one a constant-context design or cell (r4/why_pre.txt lists what POST3 printed instead: census C10–C13 C28 C29 C31 Z04 Z05, F12 F17a F17b G1 G2, CD1, SL1, H01 H02 H03 H07 H08, r26 S*A/B/G/L/O and X3 X4, K02 H07C I12, snd P1 P2a P2b P2c P4a P5 P7(-DNO_INSIDE), diff P03 P06 P06v P10b, diff2 Q1 Q3 Q5 Q6(-DSTR) Q9, snd2 P10 P2 P5, diff3 G1 G2 G3(-DM9) P1(-DCONSTS), snd3 p3 p5 p6). Runtime cells hidden in those designs because PRE refuses a constant in the same file are measured through run-time twins below.
  - = POST3 only: 185 — every one a run-time cell or a run-time refusal (r4/why_post3.txt: census A/S/W/X/V/E/C runtime, F01–F16, L1–L8, H04 H05 H06 K07, repro, snd P3 P9, diff P01 P02 P04 P05 P07(-DLETT) P08 P08b P09 P10c P11, diff2 Q2 Q4 Q5 Q7, snd2 P1 P2(-DNOK) P3 P4 P7 P8 P9, diff3 H1 H2 P1 P3(-DIV), snd3 p1 p4).
  - mixed (12): s580/r2/r26/X1, lens_snd P7, lens_diff P03/P08b/P10b/P10c (-DIV), lens_diff2 Q8 (6 variants) — each line is either PRE's (constant cells: L*, G*, OV, K6/K7 refusals) or POST3's (run-time cells: R*, W*, Q0*, H0*, F*, runtime refusals); the only lines matching neither are the summary `errors=N` counts, which add PRE-type and POST3-type refusals.
  - Exceptions: none. No diagnostic-text difference appeared in any constant cell (every = PRE variant is byte-identical, so the `inside` rendering in `binop_text`/`bin_op_text` was not reached by any measured refusal).
- Run-time twins (r4/rtonly.py strips constant-context wildcard declarations and every line naming them; r4/rt/): 42 twin variants — 19 = POST3 only, 21 both (incl. twins the stripper broke, identical errors on all three), 2 = PRE (lens_diff2 Q3: its remaining cells are constant — a package localparam and generate-ifs the stripper did not remove). Hand twin r4/rt/hand_snd.sv (snd P1 R1–R9 / Q2–Q10, P5/P7 fill F1–F3 / FQ, P2a SC1/SC2): POST4 md5 = POST3 md5.
- Report repro (cells a–i, with and without -DTWO_STATE): POST4 = POST3.
- IR identity POST3 → POST4 (vcmp + velab): 25-design battery — 24 byte-identical `.vu` and `.velab`; F12 (the constant-context fixture) is refused by `velab` on POST4 as on PRE (errors=6, no `.velab`). r3/irc single-cell designs: 228 / 228 `.velab` byte-identical.
- Tests vs other binaries (target/debug/vita swapped, restored, md5 verified): vs PRE 20 FAILED / 6 ok (backends_agree, the control, the run-time residue, and the three constant PRE pins); vs POST3 3 FAILED = exactly `constant_contexts_keep_pre_refusal`, `constant_context_residue_keeps_pre_values`, `generate_case_labels_keep_pre_semantics`.

## R4 gate
- run on the final tree (gate/run7 = current gate dir): `cargo fmt --all -- --check` rc=0; `cargo clippy --workspace --all-targets --locked -- -D warnings` rc=0 (78 s); `cargo nextest run -p cli -p elaborate -p hdl-parser -p hdl-ast --locked --no-fail-fast` rc=0 `Summary [24.627s] 8049 tests run: 8049 passed, 1 skipped`, 0 FAIL/TRY-FAIL/TIMEOUT/SIGSEGV/SIGABRT/ABORT/LEAK-FAIL/SLOW; `cargo nextest run -p sim-engine --locked --test const_domain_semantics --no-fail-fast` rc=0 `15 tests run: 15 passed`.
- The run before (gate/run6) had clippy rc=101: `doc_lazy_continuation` ×2 at inside_wildcard.rs:636/637 (a paragraph after a doc list); fixed with a blank `///` line. That run's full workspace `cargo nextest run --workspace --locked --no-fail-fast` was rc=0 `8915 tests run: 8915 passed, 15 skipped` (same code, the doc comment aside).

## POST4 freeze + corpus
- s580/post4/vita md5=4ace7617440041d0d16aee5309fded10, 7304736 bytes. `git diff --stat e6c9cc8d`: 11 files changed, 328 insertions(+), 522 deletions(-).
- `cargo run -p corpus-runner --locked -- run` rc=0, wall 364 s: sha256 ok 1.184s · aes ok 2.533s · picorv32 ok 4.245s · darkriscv ok 3.817s · biriscv ok 3.980s · serv ok 7.289s · verilog-axi ruled-split (ROADMAP §2-N) · verilog-ethernet ok 2.216s · ibex ok 29.775s · keccak ok 4.050s · keccak-arr ok 12.384s · coverage 11/11.
