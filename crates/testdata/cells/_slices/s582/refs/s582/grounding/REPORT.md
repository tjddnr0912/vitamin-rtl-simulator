# Grounding: case-inside (s582)

Status: complete (see end)

## Q1 Row repro at HEAD (PRE md5 4ace7617...)

Cell `cells/q1_repro.sv`: task sets v, `case (v) inside 4'b1?00: m=1; [4'd1:4'd3]: m=1; default: m=0; endcase`, 7 values.

vita PRE (rc=1):
```
q1_repro.sv:7:7: warning[VITA-W2004] W-PARSE-SELECT-BASE: a bit/part select here applies to an expression, not to a net or variable ...
q1_repro.sv:6:7: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected ':' in case item, found '4'b1?00'
q1_repro.sv:6:7: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected statement, found '4'b1?00'
q1_repro.sv:7:7: error[VITA-E2002] E-PARSE-UNEXPECTED-TOKEN: expected expression, found '['
errors=3 warnings=1 notes=0
```
iverilog 13.0 (rc=5): `:5: syntax error` / `:6: error: Incomprehensible case expression.` / `:7: syntax error` / `:7: Syntax in assignment statement l-value.` / `:7: error: Incomprehensible case expression.`
sv2v 0.0.13 -> iverilog (rc=0) and verilator 5.052 (rc=0), identical:
```
v=1000 m=1 / v=0010 m=1 / v=0110 m=0 / v=1100 m=1 / v=0000 m=0 / v=0011 m=1 / v=0100 m=0
```
sv2v desugar (raw): `if ((v | 4'b0100) == 4'b1100) m = 1; else if ((4'd1 <= v) && (4'd3 >= v)) m = 1; else m = 0;`
=> sv2v -> iverilog IS a working (4-state) oracle for case inside (row said only verilator); it re-reads `v` per item (no hoist).

## Q2 IEEE 1800-2017 rules (quoted from recall of the 2017 text; no local copy of the standard exists on this machine — `mdfind`/`find` found none. Tool cells below decide every point that matters.)

Grammar A.6.7: `case_statement ::= ... | [unique_priority] case ( case_expression ) inside case_inside_item {case_inside_item} endcase`;
`case_inside_item ::= open_range_list : statement_or_null | default [:] statement_or_null`;
`open_range_list ::= open_value_range {, open_value_range}`; `open_value_range ::= value_range`; `value_range ::= expression | [ expression : expression ]`.
=> only the keyword `case` (not casez/casex) takes `inside`; `matches` is the casez/casex-capable form (§12.6). No `inside` in `case_generate_construct` (§27.5).

§12.5.4: "The keyword inside can be used after the parenthesized expression to indicate a set membership case statement. In a case-inside statement, the case_expression shall be compared with each case_item_expression (open_range_list) using the set membership inside operator. The inside operator uses asymmetric wildcard matching (see 11.4.13). Accordingly, the case_expression shall be the left operand, and each case_item_expression shall be the right operand. The case_expression and each case_item_expression in braces shall be evaluated in the order specified by a normal case, unique-case, or priority-case statement. A case_item shall be matched when the inside operation compares the case_expression to the case_item_expressions and returns 1'b1 and no match when the operation returns 1'b0 or 1'bx. If all comparisons do not match and the default item is given, the default item statement shall be executed."
Example in §12.5.4: `priority case (status) inside 1, 3 : task1; 3'b0?0, [4:7]: task2; endcase // priority case fails all other values including 'b00x 'b01x 'bxxx`.

§11.4.13: "The set of values on the right-hand side ... is a comma-separated list of expressions or ranges. If an expression in the list is an unpacked array, its elements are traversed by descending into the array until reaching a singular value. ... The inside operator uses the equality ( == ) operator on nonintegral expressions ... Integral expressions also use the wildcard equality (==?) operator so that an x or z bit in a value in the set is treated as a do-not-care in that bit position (see 11.4.6). As with wildcard equality, an x or z in the expression on the left-hand side of the inside operator is not treated as a do-not-care." ... "If no match is found, but some of the comparisons result in x, the inside operator shall return 1'bx." ... "A bound specified by $ shall represent the lowest or highest value for the type of the left-hand expression. A match is found if the left-hand expression is inclusive within the range. When specifying a range, the expressions shall be of a singular type for which the relational operators (<=, >=) are defined. If the bound to the left of the colon is greater than the bound to the right, the range is empty and contains no values."

§12.5 (normal case): "The case_expression shall be evaluated exactly once and before any of the case_item_expressions. The case_item_expressions shall be evaluated and then compared in the exact order in which they appear." and the sizing rule "the length of all the case_item_expressions, as well as the case_expression, shall be made equal to the length of the longest ... If any of these expressions is unsigned, then all of them shall be treated as unsigned." — stated for the exact `===`-style case compare. §12.5.4 instead says each item is compared "using the set membership inside operator", whose operands are sized pairwise like `==?`/`<=`/`>=` (Table 11-21 operand rule `max(L(i),L(j))`, §11.8.1 signed iff both). Whether §12.5's collective width/sign applies to case-inside is not stated in §12.5.4: measured below (Q6 sizing cells) — the answer decides between "per item" and "collective".

§12.5.3 unique/unique0/priority: unique/unique0 — case_item_expressions "may be evaluated in any order and compared in any order"; a violation report if more than one item matches (unique, unique0) or if none matches and there is no default (unique, priority; not unique0). priority — first match in order; violation if no match and no default. §12.5.3.2: violation reports are deferred (observed region, glitch-free).
x/z in the case expression: §11.4.13 + §11.4.6 => in a position where the item has a 0/1 bit, the compare is x => "no match" for that item; in an item's x/z/? position the case-expression bit is don't-care (any value incl. x matches). §12.5.4's example: `'b0x0` matches `3'b0?0`; `'b00x`, `'b01x`, `'bxxx` match nothing.

## Q7 Corpus witness

Corpus RTL = `bench/<design>/src` (gitignored clones; `corpus-runner fetch`). grep `case\s*\(.*\)\s*inside` (*.sv/*.v/*.svh/*.vh) and a split-line `^\s*inside\s*$` (0 hits):

| design | case-inside lines in src/ | in the design's files.txt |
|---|---|---|
| aes, biriscv, picorv32, serv, sha256, verilog-axi, verilog-ethernet, darkriscv | 0 | 0 |
| keccak | no src/ fetched | - |
| ibex | 34, all in `src/vendor/google_riscv-dv/` (UVM instruction generator: riscv_instr.sv, riscv_b/zba/zbb/zbc/zbs/compressed/vector_instr.sv, riscv_privil_reg.sv, riscv_asm_program_gen.sv, riscv_load_store_instr_lib.sv) | 0 (ibex files.txt = src/rtl + lowrisc prim; none of the 34) |

Shapes in the 34: enum-typed scrutinee (`instr_name`, `format`, `reg_name`, `va_variant`, `supported_isa[i]`, `load_store_instr[i].instr_name`), items = enum labels, several per arm, `default` with `uvm_fatal`, inside class methods. No wildcard or range item in any of them (riscv_instr.sv:283 `R_FORMAT : ...; S_FORMAT, B_FORMAT : ...`).
=> no corpus design in CI reaches case-inside; corpus digests cannot certify it (they certify only that nothing else moved).

## Q6 batch 1 (generated cells; raw per-tool output in gen/out_<cell>_{ci,d,if}/; compact table gen/summary.txt)

Spellings: `ci` = true `case (E) inside` (oracles: sv2v->iverilog, verilator; vita PRE = E2002 page on every ci cell, or E3010 `top.inside` when the first item is a range — recovery reads `inside[lo:hi]` as a part-select of an undeclared net `inside`);
`d` = hand-spelled design-A desugar `case (1'b1) (E inside {items}): ...` on PRE; `if` = `if (E inside {..}) .. else if ..` on PRE.
On every batch-1 cell `d/vita` output == `if/vita` output (byte-identical value lines).

| cell | sv2v ci | verilator ci | PRE d == PRE if | hand-IEEE / verdict |
|---|---|---|---|---|
| c01 repro (1?00, [1:3], default; 8 values) | 1,2,0,1,0,2,0,2 | same | same | agree |
| c02 x/z/? items 1x0z,0zz1,??11 | 1,1,1,1,2,2,2,0,0,3 | same | same | agree |
| c03 x/z case expr (1x00,x100,011x,100z,xxxx,1z00,1001,zzzz,10x1) vs 1?00/0110/[8:9] | 1,0,0,0,0,1,3,0,0 | all v=0000 m=0 (2-state; not an oracle) | = sv2v | = hand-IEEE |
| c05 signed range [-4'sd2:4'sd1],[4'sd3:4'sd5] | -2..1->1, 3->2 | -2..1->0 (also 0 in verilator's own `if (v inside ..)` twin) | = sv2v | sv2v=IEEE; verilator self-contradicts (c19c: `[-2:-1]` signed works) -> disqualified on narrow signed ranges |
| c05b signed v, [4'd1:4'd3] then [-1:1] | -1->2,2->1,-7->0,0->2,1->1 | -1->0, 0->0 | = sv2v | sv2v |
| c05c unsigned v, [-1:4'sd2], [-4'sd2:4'sd2] | all 0 | all 0 | all 0 | agree |
| c06a 4-bit v vs 8-bit items 0000_1?00, 8'h1F, [5:6], 1???_??11 | 1,1,0,3,3,0 | same | same | agree |
| c06b 8-bit v vs 4'b1?00, [4'd1:4'd2] | 1,0,1,2,0,0 | same | same | agree |
| c06c signed 8-bit v vs 4'sb1?00 / 4'b1?11 | FC->1,F8->1,0C->0,08->0,FB->0,0B->2 | same | same | agree |
| c07 reversed [4'd3:4'd1] | 0,0,0 | same | same | agree |
| c08a `$` bounds unsigned | Parse error `$` | 1,1,0,2,2,0 | E3009 "`$` is only valid inside a queue element select" | PRE loud (inside op lane too) |
| c08b `$` bounds signed [$:-4'sd7],[4'sd6:$] | Parse error | -8->1,-7->1,-6->2,6->1,7->1,0->1 (unsigned reading; IEEE: -8,-7->1; 6,7->2; others 0) | E3009 | loud; no oracle |
| c09 unpacked-array item `arr` = '{5,7} | 5->0,7->0 (sv2v drops the array) | C++ compile error | E3009 "a whole unpacked array has no value in this context" | IEEE 5,7->1; both oracles fail; PRE loud |
| c10 multi items per arm | agree | agree | agree | agree |
| c11 overlap arms (first match) | 1,1,4,4 | same | same | agree |
| c12 no default, no match (m stays 9), xx00 | 1,2,9,9 | 1,2,9,9 | same | agree |
| c13a unique no-match | 1,9 (no report) | 1 then `%Error: ... unique case, but none matched for '4'h5'` + $stop | 1,9 + `W4031: value is unhandled for priority or unique case statement [at time 0]` (d); if-spelling: no W4031 (else-if chain, §3.b unique-if-chain) | report + 9 |
| c13b unique overlap (5 in [0:7] and 5) | 1,1 | `unique case, but multiple matches found for '4'h5'` + $stop | 1,1, no report (parse_unique_priority: overlap check is a documented cut; §3.b unique-overlap-note) | IEEE: report; vita PRE: none (pre-existing for plain unique case) |
| c13c unique0 no-match | 1,9 | 1,9 | 1,9, no report | agree |
| c13d priority no-match | 1,9 | `priority case, but non-match found` + $stop | 1,9 + W4031 | agree (report) |
| c13e priority overlap | 1 | 1 | 1 | agree |
| c13f unique + default | 1,2,0 | same | same | agree |
| c13g unique, x case expr x001 / 1x00 | 9, 2 | 2-state | 9 (+W4031), 2 | = sv2v / IEEE |
| c19a signed v=-1, items `-1`, `8'h00` | -1->1 | -1->0 | -1->1 | SPLIT: pairwise (sv2v) vs collective §12.5 (verilator). verilator's `if (v inside {-1,8'h00})` twin = 1 (pairwise) |
| c19b `case (a+b) inside 8'h00 / 16'h0100`, a=b=80 / 01,FF / 0,0 | 1,1,1 | 2,2,1 | 1,1,1 | SPLIT (width). verilator's `if` twin = 1,1,1 |
| c19c signed v vs [-2:-1], [4'd0:4'd1] | 1,1,2 | 0,0,2 | 1,1,2 | SPLIT (sign; collective unsigned). verilator `if` twin = 1,1,2 |
| c20a `case (4'b1100) inside 4'b1?00` | 1 | 1 | 1 | agree |
| c20b `case (P)` P=4'b1000 localparam | 1 | 1 | 1 | agree |
| c30 item = variable w=4'b1x00 (run-time x) | 1000->1,1100->1,0100->0 | 1,0,0 (2-state w=1000) | 0,0,0 | PRE SILENT-WRONG (§2 🆕 S (b) residue: run-time x/z element compared with `==`); IEEE = sv2v |
| c31 item `{2'b1?, 2'b00}` | 1,1,0 | 1,1,0 | E3009 "an `inside` element that builds an x/z/? literal into a larger expression is not supported" | loud |
| c32 72-bit v, wildcard + range | 1,0,2,0 | same | same | agree |
| c34 item = localparam P=4'b1?00 | 1,1,0 | 1,1,0 | E3009 "parameter `P` value is not a constant: 4'b1?00 has no constant-fold arm" | loud (param itself) |
| c37 `4'b????` item vs xxxx/zzzz/3 | 1,1,1 | 2-state | 1,1,1 | agree |
| c38 empty statement arm | 9,2,0 | same | same | agree |
| c39 items `1, 3` / `[4:7]` / `'1`; v=15 | 15->0 (sv2v writes `'1` as `1'sb1`; its `if` twin writes `{4{1'sb1}}` -> 3: sv2v self-contradicts, disqualified) | 15->0 (collective: `'1` at 32 bits) | 15->3 (pairwise: `'1` at 4 bits) | SPLIT (fill width): verilator collective vs vita pairwise; verilator's `if` twin = 3 |
| c40 unsized x-MSB `'bx1` vs 36-bit | 1,1,0 | same | same | agree |

Finding F3 (decides the design): on case-inside verilator sizes/signs collectively (§12.5: c19a, c19b, c19c, c39) while its own `inside` operator is pairwise; sv2v (translation = `if` chain) is pairwise. iverilog rejects. => width/sign of case-inside items is an ORACLE SPLIT wherever pairwise != collective. Every 2-state cell where the two rules coincide agrees across sv2v, verilator and the PRE `inside` operator.

## Q6 batch 2 (hand-written cells in b2/; raw outputs b2/out_*/)

| cell | sv2v ci | verilator ci | PRE d (design-A spelling) / if | hand-IEEE / verdict |
|---|---|---|---|---|
| m1 pair context: `(a+b)` vs 9'h100 (a=b=80); v8 vs `a4+b4`; v8 vs `[a4+b4:8'hFF]`; F7/07 vs `~a4` | 1,1,1,1,0 | 1,1,1,1,0 | if: 1,1,1,1,0 (and `(a+b)==9'h100` = 1) | agree; PRE `inside` op sizes each pair right |
| m2 side-effect case expr `f(n)` (f does `cnt++` + $display): 4 items/match last; 2 items/no match; default only | f called once in each (cnt=1,1,1) | once each | d: E3009 x7 "function `f` assigns a module net from its body..." (label position) | IEEE: once, before items |
| m2b pure f (only $display), same shapes | `f(3)` x1, `f(7)` x1 | x1, x1 | d: `f(3)` x4, `f(7)` x0 (SILENT-WRONG for design A); PRE plain `case (f(3))` twin: x1, x1 (iverilog x1, x1) | once — needs the lower_case scrutinee hoist |
| m3 call items `g(1), g(2): / [g(3):g(6)]: / g(5):`, v=5 | g(1)x7, g(2)x7, g(3), g(6), m=2 (sv2v duplicates) | g(1) g(2) g(3) g(6) g(5) g(1) g(2) g(3) g(6), m=2 (evaluates all, twice) | d: g(1) g(2) g(3) g(6), m=2 | §12.5 linear search: g(1) g(2) g(3) g(6), m=2 = PRE d. Plain-case twin: PRE g(1) g(2) g(5) m=3 = iverilog; verilator again evaluates all twice -> neither tool is an order oracle for case-inside; hand-IEEE |
| m4 contexts: always_comb, always_ff (arms `8'hAB`/`4'h5`/`16'hFFEE` into 8-bit r), function, task, class method, nested case inside, 5 values | sv2v: parse error (class in module) | comb/ff/fn/task/meth: 1,2,0,1,2 / ab,05,ee,ab,05; nest C,A,A,C,B | d: identical to verilator, all lanes | agree |
| m5 const function with case inside used in localparams (and `[P1+P2:0]`) | P1=1 P2=2 P3=0 bits=4 rt=1 | `Expecting expression to be constant, but can't determine constant for FUNCREF 'f'` | d and if: E3009 "parameter `P1` value is not a constant: `f(…)` has no constant-fold arm"; plain-case twin also E3009 (iverilog/verilator fold it) | loud on PRE for ANY case in a const function (const_fn.rs interpreter has no Stmt::Case arm, `_ => None` at const_fn.rs:1719) |
| m6 generate-region `case (P) inside` | sv2v `Parse error: unexpected token 'inside'`; iverilog syntax errors + `assert: pform.cc:1478` abort | `syntax error, unexpected inside` | E2002 "expected ':' in generate-case item" | illegal (A.4.1.2 case_generate_construct has no inside); stays loud |
| m6 `casez (v) inside` | `cannot use inside with casez` | `Illegal to have inside on a casex/casez` | E2002 x2 | illegal; stays loud |
| m6 `randcase` | - | m=1 | E2002 x3 (`expected '=' or '<=' after lvalue, found '1'`) | separate keyword path; out of scope |
| m7 string case expr ("aa","ab"); enum (riscv-dv shape: labels, two-label arm, `[U_F:J_F]`) | sv2v fails (`Object top.f has no method "name(...)"`) | s 1,0; f 1,0,2,2,3,3 | d: identical | agree |
| m7 real case expr `1.0`, `[2.0:3.0]`, r=2.5 / 1.0 | iverilog `^ operator may not have REAL operands` | `Internal Error ... V3Number.cpp:1910` | d: 2, 1 | no oracle; hand-IEEE (§11.4.13 `==` and `<=`/`>=` on real) = 2, 1 |
| m9 `logic [3:0] inside;` + `case (v) inside[1:0]:` | - (iverilog: syntax error) | `syntax error, unexpected inside` | PRE accepts `inside` as a variable name; m=0 | SV-illegal design; after the change the same text becomes a case-inside with the empty range [1:0] (m=0 too) |

## Q3 Code census at HEAD 60c70e05

(a) Parser, case statements
- dispatch: `crates/hdl-parser/src/stmt.rs:154-156` (`Kw::Case/Casez/Casex => self.parse_case(CaseKind::…)`); qualified: `stmt.rs:166` -> `assertions.rs:448 parse_unique_priority`.
- `crates/hdl-parser/src/stmt_ctl.rs:91 parse_case(kind)`: `(` `self.expr(0)` `)` then `parse_case_item` until `endcase`; `:115 parse_case_item`: `default [:] stmt` | `label {, label} : stmt` (labels = `self.expr(0)`).
- Why `case (v) inside` fails: `inside` is a contextual identifier (`expr.rs:191 at_ident_kw("inside")`), so `parse_case_item`'s first `self.expr(0)` reads `inside` as an identifier; next token `4'b1?00` -> `expected ':' in case item` (E2002), recovery `expected statement`; a following `[lo:hi]` item is read as a select (W2004 + `expected expression, found '['`). When the FIRST item is a range, `inside[lo:hi]` parses as a part-select label and elaboration reports `E3010 undeclared net/variable top.inside` (c07, c11, c13b, c13e, c19a). PRE accepts `logic [3:0] inside;` as a variable (m9; iverilog and verilator reject it: reserved word).
- AST: `crates/hdl-ast/src/lib.rs:998 Stmt::Case { kind: CaseKind, scrutinee: Expr, items: Vec<CaseItem>, span }`; `:1426 enum CaseKind { Case, Casez, Casex }`; `:1432 enum CaseItem { Match { labels: Vec<Expr>, body, span }, Default { body, span } }`. All derive `SchemaHash` (`lib.rs:8-19`: root `schema_hash::<SourceUnit>()` pinned in `crates/hdl-ast/tests/schema_hash.rs`; field/variant change flips it and invalidates every `.vu`). Precedent §4.5.580: `BinOp::InsideEq` APPENDED LAST (`lib.rs:1814`) => hdl-ast hash re-pinned, `format_version` 34 (`crates/vita-artifact/src/header.rs:15`) and sim-ir goldens unchanged, `.velab` byte-identical except the 32-byte upstream `.vu` digest. A `CaseKind::Inside` appended last would take the same path (re-pin only). `parse_unique_priority`'s own comment (`assertions.rs:444-447`) records that a marker field "would change the frozen AST shape".
- Other Stmt::Case producers: `enums.rs:260` (synthesized enum-method body, kind Case), `monomorph.rs:207/668` (rebuild, copies kind), generate path `generate.rs:89 parse_gen_case` -> `GenItem::Case` (separate type `GenCaseItem`, lib.rs:1984/2003; `case … inside` there stays E2002 — m6; illegal by grammar), `module_items.rs:1364` (bare module-level case -> generate).

(b) `inside` operator
- parse: `expr.rs:191-197` (relational bp 17) -> `stmt_ctl.rs:10 parse_inside(lhs)`: `{` terms `}`; value term = `Binary{InsideEq, lhs.clone(), v}` (`:40`); range term = `LogAnd(Ge(lhs.clone(), lo), Le(lhs.clone(), hi))` (`:29-31`); terms OR-folded (`:62-64`); empty set = `1'b0`. `lhs` is cloned per term (comment `:21-27`: "wants a hoist to a temporary, like the case scrutinee got"). `$` bound -> `ExprKind::Dollar` -> elaborate `expr_main.rs:1453` E3009 unless `dollar_subst` is set (queue index only, `dynarr.rs:473`).
- lower: `expr_main.rs:484-562` Binary arm (string route `:518-546` for `InsideEq` with a string operand -> StrCmp; handle route `:494-516`; `:554-557` `inside_value_cmp(lhs, rhs)` else `Eq`); ctx twin `expr_ctx.rs:846-872` (`is_cmp` includes InsideEq; fill element sized to the sibling; `:869-870` inside_value_cmp). `wildcard_eq.rs:86 inside_value_cmp(lhs_id, el_id)` (table at `:73-79`: real -> `==`; provably no x/z -> `==`; one `Const` with x/z -> `wildcard_cmp_ids` `:193`; x/z literal inside a larger expr -> loud E3009; variable/call -> `==` = §2 🆕 S (b) residue). Its module doc (`wildcard_eq.rs:1-5`) names it "the element rule a `case … inside` item (§12.5.4) is to reuse".
- constant folds read InsideEq as Eq: `const_eval.rs:1527`, `const_fn.rs:131`, `const_wide.rs:213/228/852/894/1157`, `const_real.rs:194`, `const_str.rs:172-178`, `block_local/proofs.rs:263`, `crv.rs:43/150/266` (constraints), `const_fn_width.rs:107`, `const_bound.rs:947`, `const_level_header.rs:577` (text).

(c) Elaborate case lowering and every Stmt::Case consumer
- ONE lowering dispatch: `stmt_main.rs:772-777` -> `stmt_flow.rs:651 lower_case(b, kind, scrutinee, items, span)`: labels lowered once (`:679 lower_case_label` = `expr_special.rs:855`, fill sized to the selector via `sibling_ctx`); §12.5 fill re-size pass (`:703-740`); §12.5 one-region ctx pass (`:766-786`, `case_operand_takes_ctx` `:601`, `lower_case_operand_ctx` `:644`); COLLECTIVE sign -> `$unsigned` scrutinee (`:801-814`); scrutinee hoist `hoist_case_scrutinee` `:516` (skips real/string/Const/no-width; module scope `fresh_case_tmp` `:233` = `$ia_tmp$` Reg net with the scrutinee's sign; frame bodies use a slot reserved by `frames_reserve.rs:614 reserve_frame_case_tmps` keyed on the case span, miss => per-arm re-evaluation); cascade `:852-862` `case_cmp` (`expr_special.rs:894`: string scrutinee + kind Case -> StrCmp==0; else `CaseEq`/`CasezEq`/`CasexEq` — EXHAUSTIVE match on CaseKind at `:910-914`, a new variant is a compile error there). CaseKind is read at exactly these two elaborate sites (grep `CaseKind` in crates/elaborate/src).
- casez/casex wildcard: engine ops `CasezEq`/`CasexEq` (sim-ir); no elaborate masking.
- unique/priority: parser-only (`assertions.rs:448-509`): no default + not unique0/priority0 => appends `CaseItem::Default { $__vita_unique_violation("value is unhandled …") }` -> `SeverityKind::UniqueViolation` (W4031). Overlap ("multiple matches") is NOT checked (`assertions.rs:436-438`, "documented cut"). Kind is passed through `parse_case(kind)` (`:484-489`); nothing else reads it.
- AST walkers that see Stmt::Case (all ignore `kind`; all walk labels as plain Exprs; none interprets a label's value): multidriver.rs:108,202; ports.rs:132; frames_classify.rs:21,240,836,859; frames_classify_write.rs:277,348; frames_reserve.rs:229,647,709 (`collect_case_spans`); ast_query.rs:141 (callee collect); package.rs:115 (rewrite); pkg_scoped_frames.rs:363; proc_builder.rs:145; hier.rs:36,76; block_local_class.rs:713; block_local/{mod.rs:57,130; proofs.rs:74; hoist.rs:819}; hoist/{mod.rs:460; general_stmt.rs:118 (hoists SCRUTINEE calls only, items cloned); special.rs:531 (same)}; da/{mod.rs:256,488; writes.rs:526; loops.rs:54; reads.rs:213,301,780,861}; const_level_header.rs:368; sva_decl.rs:279 (rebuild); sva_ast.rs:166; parser monomorph.rs:207,668. always_comb/always_latch sensitivity = `ast_query.rs:562 comb_read_set` over the LOWERED IR (`ir::BasicBlock`), so it sees whatever lower_case emits.
- const-function interpreter (`const_fn.rs:1595-1720`): no `Stmt::Case` arm (`_ => None`) => any case in a constant-context call is E3009 on PRE (m5, plain-case twin included).
- backends: lower_case emits only BlockingAssign (hoist) + `Terminator::Branch` + existing IR Binary ops; no CaseKind reaches sim-ir; interp/vm/native consume the same IR (backend_equiv). VCD/FST: `$ia_tmp$` nets filtered by name prefix (`queues_io.rs`, comment at `stmt_flow.rs:216-219`).

(d) `parse_unique_priority`: `assertions.rs:448`; kinds at `:484-489` map Kw -> CaseKind and call `parse_case(kind)`; a case-inside branch placed INSIDE `parse_case` (after `)`, only when kind == Case and `at_ident_kw("inside")`) reaches the qualified form with no change here. casez/casex followed by `inside` must stay an error (m6: both oracles reject it).

(e) Error recovery: `stmt_ctl.rs:132` `self.expect(Colon, "':' in case item")` (1st E2002), `parse_statement` on the stray item token ("expected statement", 2nd E2002), and a range item `[` -> the select path (`W2004 W-PARSE-SELECT-BASE` + "expected expression, found '['"); with a range FIRST, `inside[lo:hi]` is a valid label -> E3010 at elaborate.

## Q8 Siblings (noted only)

- `casez/casex (e) inside`: illegal by A.6.7 (only `case` takes `inside`); sv2v `Parse error: cannot use inside with casez`, verilator `Illegal to have inside on a casex/casez`, iverilog syntax error; PRE E2002 x2 (m6). Same parser function (`parse_case`); the new branch must accept `inside` only for `CaseKind::Case` and keep casez/casex + inside an error.
- generate-region `case (P) inside`: illegal (§27.5 grammar); all three oracles reject (iverilog aborts `pform.cc:1478`); PRE E2002 via `generate.rs:89 parse_gen_case` — a different parser path, untouched by a `parse_case` change.
- `randcase` (§18.16): no parser support (PRE E2002 `expected '=' or '<=' after lvalue, found '1'`; it is not the `case` keyword path); iverilog rejects; verilator runs. Out of scope.
- `case (x) matches` (§12.6): PRE fails earlier (tagged unions unsupported: `expected packed after union`), then `expected ':' in case item, found identifier 'tagged'`; iverilog syntax errors; verilator `Unsupported: case matches (for tagged union)`. No oracle; out of scope. Note `matches` would reach the same `parse_case` spot as `inside` (a contextual word after `)`).

## Q6 batch 3: sign/width pair matrix (mx/; generator mx/gen_mx.py, model + guard check mx/model.py, result mx/analysis.txt)

78 designs = 6 case expressions {u4 leaf, s4 leaf, s8 leaf, u8 leaf, u8 `a+b`, s4 `a+b`} x 13 items {`-1`, `-4'sd2`, `8'hFE`, `[-2:-1]`, `[-4'sd3:-4'sd1]`, `[4'd14:4'd15]`, `4'sb1?10`, `4'b1?10`, `'1`, `16'h0100`, `8'sh80`, `8'hFC`, `[8'hFC:8'hFE]`}, each under 4 never-matching siblings {none, `8'h55`, `16'sh7777`, `32'd12345`} = 312 rows, 8-16 values each. ci on sv2v + verilator; `if (E inside {…})` twin on PRE + sv2v.
Model: P = per pair (width max of the pair, signed iff both, operators evaluated at the pair width, a fill at the case expression's width); C = §12.5 collective (width max over all, signed iff all).
- PRE `if`-twin == P on all 312 rows, every value (0 mismatches).
- P != C on 63 rows; on every one sv2v = P, and verilator = C (except E3s8 x `4'sb1?10` rows, where verilator follows its own rule below).
- verilator's case sizing is a third rule V: width collective, case expression extended by the collective sign, each ITEM extended by its OWN sign. Raw (b3/out_vx_twins): `A ci u8 vs 8'sh80 +16'sh7777 m=0`, `A pc` (plain case) `m=0`, `A op` (inside op) `m=1`; `D ci u8=0E vs 4'sb1?10 m=0` vs its own `D weq u8=0E ==? 4'sb1?10 m=1` and `D op m=1`; iverilog plain case `A pc m=1`, `B pc m=1`, PRE plain case `A pc m=1`, `B pc m=1`. => verilator self-contradicts on signed-item extension (case-inside vs its own `==?`/inside; plain case vs iverilog — the same deviation §2 🆕 T records as W06/T07p) -> disqualified on that sub-axis.
- sv2v writes a case-inside fill `'1` as `1'sb1` (c39, mx I09) and its own `if` twin as `{4{1'sb1}}` -> disqualified on fills.

Guard (candidate split rule, accept iff both hold; mx/model.py `guard`):
  sign_ok  = case expression unsigned, OR every item/bound signed, OR no item/bound signed
  width_ok = every context-sensitive participant (operator: unary -,+,~, binary arith/bitwise/shift, ternary; fill `'0/'1/'x/'z`) sits in a pair whose width equals the case-wide maximum W (case expression: own width == W; item k: max(w_E, w_k) == W)
Result over the 312 rows: accepted 174, refused 138; accepted rows with P != C: 0; refused rows where P == C on every value: 75 (over-refusal; those stay loud as on PRE).
On the 174 accepted rows the remaining oracle disagreements are only the disqualified ones: verilator on `4'sb1?10` / `8'sh80` (28 rows, rule V) and sv2v on `'1` (6 rows).

## Q6 batch 4 (b3/): lanes the design routes into

| cell | oracles | PRE | note |
|---|---|---|---|
| f1 frame hoist: `case (g(n)) inside` in function / task / class method, g prints | verilator: g(3) once in each, m=4 (sv2v: class-in-module parse error) | plain-case twin f1b (function, task): g(3) once each, m=4, native == interp == vm | module-function frame lane hoists |
| f2 class-method plain `case (g(n))` (g a class method) | iverilog: g(3) x1, m=4; verilator x1, m=4 (ci twin: x1, m=4) | g(3) x4, m=4 | PRE PRE-EXISTING SILENT-WRONG (2 oracles): the class-method lane misses the scrutinee hoist and re-evaluates per tested label |
| s2/s3 string-returning call as case expression `case (sf(1)) "b":` | verilator m=3; iverilog aborts (`draw_eval_vec4` assertion) | m=0 (sf printed once) | PRE PRE-EXISTING SILENT-WRONG (1 oracle) in plain case; `t = sf(1); case (t)` = 3 and `if (sf(1) inside {..})` = 3 (sf x3) on PRE |
| s1 string range `["aa":"az"]` | verilator Internal Error (`INSIDERANGE` / V3Number) both spellings; sv2v n/a | d: am->1, b->2, ba->0, a->0 | no oracle |
| v2 variable items `bit [3:0] k2`, `int ki`, `logic [3:0] k4` (no x), range `[k2+4'd5:4'd9]` | sv2v = verilator: 0,0,0,1,0,2,0,3,3 | d: identical | agree |
| a1 always_comb sensitivity to range-bound variables lo/hi | sv2v = verilator: t1 0, t2 1, t3 0, t4 2 | d: identical; plain-case twin (labels lo, hi) = iverilog = verilator | agree |
| backends | - | 13 d-spelling cells: native == interp == vm byte-identical (be/) | |

## Q4 Candidate designs

(A) Parser desugar `case (E) inside` -> `case (1'b1) (E inside {items_k}): …` (or an if/else chain), reusing `parse_inside`'s terms with E cloned per term.
- Edits: `parse_case` (stmt_ctl.rs:91). No AST change. Routes into: `parse_inside` term shapes; `lower_case` (scrutinee `1'b1` is a Const -> no hoist; §12.5 passes are no-ops on 1-bit labels); Binary InsideEq/Ge/Le/LogAnd lowering (`expr_main.rs:484-562`, `expr_ctx.rs:846-872`); `inside_value_cmp`; `wildcard_cmp_ids`; `parse_unique_priority` default injection.
- Measured: PRE d == PRE if on every cell; right on every agreeing cell.
- Fails: (1) evaluate-once: m2b `f(3)` x4 and default-only `f(7)` x0 where both oracles print x1/x1 (silent); m2 (f writes a module net) E3009 x7. (2) width/sign is always per pair: 63/312 matrix rows plus c19a/b/c, c39 are oracle splits that the parser cannot see (no widths at parse time) — ships one side of a split (ER §4.6). (3) inherits 🆕 S (b) silently (c30). => not correct-or-loud. Rejected.

(B) Append `CaseKind::Inside`; parser emits `Stmt::Case { kind: Inside, scrutinee: E, items }` where every open_value_range is one label in the shape `parse_inside` already builds, over an inert placeholder left operand (`ExprKind::Dollar`): value `Binary{InsideEq, $, v}`, range `LogAnd(Ge($, lo), Le($, hi))`. `lower_case` dispatches `Inside` first to a new `lower_case_inside`, which lowers only v/lo/hi (never the placeholder), applies the guard, hoists via `hoist_case_scrutinee`, and builds each test with `inside_value_cmp(tmp, v)` else `Eq` (string: StrCmp == 0, as `case_cmp`), range `Ge && Le`, branch on `test === 1'b1`.
- Edits: hdl-parser `parse_case` (new branch, only `kind == Case` + `at_ident_kw("inside")` after `)`); hdl-ast `CaseKind` (+`Inside`, appended; SchemaHash re-pin in `crates/hdl-ast/tests/schema_hash.rs`, no format_version bump per §4.5.580 precedent); `lower_case` (early dispatch); `case_cmp` (exhaustive match needs an arm, unreachable); new `lower_case_inside` + guard.
- Placeholder property: every AST walker treats `Dollar` as an inert leaf (`da/reads.rs:403`, `da/writes.rs:45`, `frames_classify.rs:114`, `hoist/general.rs:144`, `gen_enum.rs:287`, `inline_body_ctx.rs:107`, `strings.rs:34`, `arrays.rs:858`, `scope.rs:353`, `const_eval.rs:1239`); any lane that lowered the label generically would hit `expr_main.rs:1453` E3009 (dollar_subst is only set while lowering a queue index, `dynarr.rs:473`) — loud, not silent.
- Cost: parser ~40 lines, elaborate ~150 lines, one hdl-ast hash re-pin, a pin file, manual 003 row (`docs/manual/003_language-reference.md:758`), CHANGELOG.

(C) A new `CaseItem` variant or a new range `ExprKind`/`BinOp`: every exhaustive CaseItem walker (~40 sites, Q3c) or every expression walker needs an arm; a `..`/catch-all walker silently drops the bounds (always_comb sensitivity, DA reads). Rejected: higher cost and risk than (B) for the same semantics.

## Q5 Lane table for (B) (ER §10.2)

| # | shared function (callers) | edit / route | consumer lane | status | evidence |
|---|---|---|---|---|---|
| 1 | `parse_case` stmt_ctl.rs:91 (stmt.rs:154-156 x3 kinds; assertions.rs:489) | EDIT | case/casez/casex not followed by `inside` | opted-out: branch keyed on `kind == Case && at_ident_kw("inside")` after `)`, AST byte-identical | confirm at POST: `.vu`/`.velab` identity on suite + corpus |
| 1 | | | `case (E) inside` unqualified | measured | batches 1-4 |
| 1 | | | unique / unique0 / priority + inside | measured | c13a-g |
| 1 | | | casez/casex + inside (must stay loud) | measured | m6 |
| 1 | | | first label is an identifier named `inside` | measured | m9 (both oracles reject the declaration; 0 declarations in tests/corpus) |
| 2 | `parse_inside` stmt_ctl.rs:10 (expr.rs:196) | not edited (own item parser) | every `inside`-operator lane | opted-out | - |
| 3 | hdl-ast `CaseKind` (frozen; `case_cmp` exhaustive match, monomorph copy, `.vu` postcard) | EDIT (append `Inside`) | existing ASTs | opted-out: appended variant, discriminants unchanged; hash re-pin only (§4.5.580 precedent) | `crates/hdl-ast/tests/schema_hash.rs` |
| 3 | | | staged vcmp -> `.vu` -> velab -> vrun of a case-inside design | UNMEASURED (the new construct cannot exist on PRE) | measure in-slice like `crates/cli/tests/staged_flow.rs` |
| 4 | `lower_case` stmt_flow.rs:651 (stmt_main.rs:777; every case in every context) | EDIT (dispatch `Inside` first) | Case/Casez/Casex | opted-out: early return keyed on kind, IR byte-identical | POST `.velab` identity |
| 5 | `hoist_case_scrutinee` stmt_flow.rs:516 | route | module-scope process | measured | m2, m2b (oracles x1); PRE plain-case twin x1 |
| 5 | | | function/task frame (`reserve_frame_case_tmps` frames_reserve.rs:614; `collect_case_spans` kind-blind) | measured | f1 (verilator x1); f1b PRE plain case x1 on native/interp/vm |
| 5 | | | class-method body | measured: PRE plain case MISSES the hoist (g x4; iverilog/verilator x1) -> refuse a side-effecting E whenever the hoist returns the input id | f2 |
| 5 | | | Const case expression (no hoist) | measured | c20a, c20b |
| 5 | | | string variable (no hoist; StrCmp) | measured | m7b (verilator) |
| 5 | | | string-returning call | measured: PRE plain case m=0 vs verilator 3 -> refuse unless `ir_expr_is_string(scrut)` | s2, s3 |
| 5 | | | real case expression / real items | measured: no oracle (verilator internal error; sv2v -> iverilog rejects) -> refuse | m7 |
| 6 | `inside_value_cmp` wildcard_eq.rs:86 (expr_main.rs:555, expr_ctx.rs:870) | route | lhs = hoisted tmp; Const element with x/z | measured | c02, c03, c06a-c, c32, c37, c40, mx I07/I08 |
| 6 | | | 2-state or x/z-free constant element | measured | c01, c10, v2, m7b |
| 6 | | | non-constant 4-state element (run-time x/z) | measured: PRE `==` silent (c30) -> refuse in case-inside | c30 |
| 6 | | | compound x/z element | measured: existing E3009 | c31 |
| 7 | `wildcard_cmp_ids` wildcard_eq.rs:193 (==?, !=?, inside) | route via 6 | widths 4-72, both signs | measured | c06a-c, c32, mx |
| 8 | `expr_may_be_unknown` | route (classifier for the 6 refusal) | 4-state var / 2-state var | measured | c30, v2 |
| 9 | `ir_bits_of`, `expr_self_signed`, operator shape, `expr_contains_fill` (the predicates lower_case already uses, stmt_flow.rs:756-806) | route (guard inputs) | leaves, unary minus, binary operators, fills, unsized ints, 4-32 bit, both signs | measured | mx 312 rows: 0 accepted rows with P != C; c19a-c, c39, c05* |
| 10 | `parse_unique_priority` assertions.rs:448 | route (unchanged) | default-arm injection (W4031) | measured | c13a-g |
| 11 | IR Eq/Ge/Le/LogAnd/CaseEq + Branch; interp/vm/native | route | 3 backends | measured | be/: 13 cells byte-identical |
| 12 | `comb_read_set` ast_query.rs:562 (lowered IR) | route | always_comb with variable bounds | measured | a1 |
| 13 | const-function interpreter const_fn.rs:1595 (no Stmt::Case arm) | route | case-inside in a constant-context call | opted-out: stays E3009 (plain case too) | m5 |
| 14 | `parse_gen_case` generate.rs:89 | untouched | generate-region case inside | opted-out: stays E2002 (illegal) | m6 |
| 15 | AST walkers (Q3c list) | route (labels walked as Exprs; `$` inert leaf) | each | measured on the d spelling (labels = superset: E cloned + same items) in comb/ff/function/task/class/nested/frames/sensitivity cells; Dollar arms read (Q4 B) | m4, f1, a1, v2 |

Unmeasured lanes: 1 (the staged `.vu` round trip of the new kind; POST only).

## Risks (paths where a case-inside node could be silently mis-lowered)
1. Dispatch placed after lower_case's §12.5 passes (fill re-size `:703-740`, ctx re-lower `:766-786`, collective `$unsigned` `:801-814`): the scrutinee would be wrapped/re-lowered by the whole-case rule before the per-pair compare. Dispatch at the top of lower_case.
2. Hoist misses (class-method bodies, f2: PRE x4 vs oracles x1; frame owner mismatch documented at stmt_flow.rs:556-570) + side-effecting E -> extra evaluations at exit 0.
3. String-returning call as E: `case (sf(1)) "b":` is m=0 on PRE (verilator 3) because `ir_expr_is_string` is false for the call; the same path would serve case-inside.
4. Non-constant 4-state value items: `inside_value_cmp` returns None -> `==` -> x -> no match (c30; §2 🆕 S (b)).
5. The split: an operator the guard classifies as a leaf picks one side of a measured oracle split silently. Use a POSITIVE leaf set (identifier, literal, select, concat/replication, call, folded Const); anything else must sit at the case-wide width.
6. Placeholder via `dollar_subst` instead of structural lowering: every bare `$` in an item or bound (`[lo:$]`, `arr[$]` on a fixed array) would become the case expression. Lower only v/lo/hi.
7. Branch on the raw test: the test is 1/0/x; match must be `test === 1'b1` (IEEE: x is no match), as the d spelling's `CaseEq(1'b1, test)` does (c03, c12, c13g).
8. `unique case … inside` overlap: no report (documented cut, assertions.rs:436-438; §3.b unique-overlap-note); verilator reports `multiple matches`. Values right; diagnostic gap pre-existing for plain unique case.
9. `case (x) inside: …` with a V2001 identifier `inside` changes meaning (SV-illegal; m9).
10. A later const_fn.rs `Stmt::Case` arm would need an Inside branch; nothing forces it (const_fn matches on Stmt, not CaseKind).
11. Fill items: lowered at E's width (pairwise); with a wider participant the oracles split (c39) — refuse.

## Pre-existing defects found (outside the slice; catalogue)
- P1 class-method lane: plain `case (g(n))` evaluates g once per tested label (b3/f2: PRE x4, iverilog x1, verilator x1). 2-oracle silent-wrong.
- P2 plain `case (sf(1)) "b":` with a string-returning function: PRE m=0, verilator m=3, iverilog aborts (b3/s3). 1-oracle silent-wrong.
- P3 `inside` operator with a side-effecting left operand evaluates it once per element (b3/s3: `sf(1)` x3, verilator x1) — the parse_inside clone noted at stmt_ctl.rs:21-27.
- P4 verilator 5.052 narrow signed `inside` ranges compare unsigned (c05: `-2 inside {[-4'sd2:4'sd1]}` = 0 in both spellings) — oracle defect.

Status: grounding complete.
