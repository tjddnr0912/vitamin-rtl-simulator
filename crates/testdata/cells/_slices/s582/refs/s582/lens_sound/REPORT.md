# lens:soundness round 1 — s582 case-inside
status: DONE — verdict FINDINGS (0 BLOCKING, 2 NON-BLOCKING), product_shakes: no
## questions
(updated after each question)

### Q3a operator items narrower than E (census m1 p3-p6 were never run on POST: the m1 design was refused whole by p1)
design d/d02_opitem_narrow_2state.sv (bit operands): POST = sv2v->iverilog on 16/16 lines (a4+b4, [a4+b4:FF], a4<<1, ~a4, {a4+b4}, signed sa+sb, -sa, [sa+sb:0], signed'(sa+sb)). verilator build fails on the if-twins (own limitation). 4-state operands (d/d01): value items refused clause 5; a 4-state operator RANGE bound accepted (line 23) — ranges are <=/>=, no wildcard: correct by construction.
verdict: no finding.

### Q4a parser `inside` branch vs a pre-existing identifier `inside`
design d/d03_v2005_inside_ident.v (legal 1364-2005: `reg [3:0] inside;` then `case (x) inside[1:0]: …` and `case (x) inside + 4'd0, 4'd9: …`)
PRE: A m=1, B m=1 (rc=0) · iverilog 13 default generation: A m=1, B m=1 · POST: A m=0, B m=0 (rc=0, no diagnostic)
control d/d03b: PRE rejects `reg [3:0] logic;` / `bit` (E2002) — vita applies SV keyword rules except for `inside` (lexed as identifier, `at_ident_kw`).
plan m9 measured the same shape but its values gave PRE m=0 = POST m=0, which hid the value change.
FINDING F1 (see findings).

### Q2a AST-hoisted writer call as E (capture type through `hoist_inout_general_top`)
design d/d04b_hoisted_signed_call.sv: `function automatic int f` writes `cnt`; `case (f(-1)) inside [-2:1]`
POST A m=1 cnt=1, A2 m=1 cnt=2 · sv2v->iverilog A m=1 cnt=1, A2 m=1 cnt=2 · verilator A m=0, A2 m=0 (verilator's own `if (f(-1) inside {[-2:1]})` twin = 1 in d/d04: self-contradiction, disqualified on this cell)
d/d04 (with an output-arg call g): POST B m=1 o=-1; sv2v cannot run (iverilog: function output port). No finding. Recorded: POST `if (f(-1) inside {[-2:1]})` runs f twice (cnt 3->5) = P3 (pre-existing, parse_inside clone).

### Q2b contexts (frame/inline/class) — no slice finding
design d/d05d_inline_class_ctx.sv: automatic function from `assign` (cf), static function with operator E `v + 4'd1` (cs), class method with argument E: POST = verilator on 5/5 lines (t1 10/11, t2 20/21, t3 20/21, meth=1, meth2=2). PRE: `inside[lo:hi]` parsed as a part-select of an identifier `inside` -> E3010 (this is F1's mechanism: PRE reads `case (x) inside[...]` as a plain case on a net named `inside`).
d/d05c: class method `case (this.k) inside` -> one E3009 clause 6 (hoist miss + `expr_is_repeatable` fail-closed on a handle read): over-refusal, loud.
### Q3b guard inputs (code read)
- `assign_pattern_expr_has_call` arrays.rs:849 — `_ => true` (positive set of call-free kinds): fail-closed. `expr_may_be_unknown` packed.rs:1153 — missing id -> true; Signal word:Some -> true; 2-state whole read -> false: fact-or-refuse. `expr_is_repeatable` packed.rs:1274 — ArrayItem/Signal word:Some -> false; SysFunc positive list: fail-closed.
- `canonical_self_width` None -> refuse; `ir_bits_of`/`expr_self_signed` compared to canonical for E only (items never read through expr_self_signed).
- `inside_operator`: AST (Paren, MinTypMax stripped) OR IR top node; IR BinOp/UnOp matches are exhaustive (new op = compile error). IR kinds Concat/Select/SysFunc/Call/ArrayItem/Signal/Const are self-determined, so "leaf" is the correct answer there; d/d02 measured `{a4+b4}` and `signed'(sa+sb)` = sv2v.

### Q5 staged `.vu` (frozen POST/PRE `vita vcmp|velab|vrun`)
vu/plain.sv PRE vcmp -> POST velab: rc=2 E9002 "sim-ir type shape changed between builds; rerun `velab`" (loud stale). vu/ci.sv POST vcmp -> PRE velab: rc=2 E9002. POST vcmp->velab->vrun of a case-inside design: `m=1`, rc=0 (= one-shot). No finding (message names `velab` for a `.vu` written by `vcmp`: wording, pre-existing).

### Q3c domain of E (string/real/struct member) — no slice finding
d/d06_domain_e.sv POST: `case (sarr[1]) inside "b"` and `case (sq[0]) inside "b"` -> E3009 clause 6 (string element: not captured, `Signal word:Some` not repeatable) — loud; real E -> E3009 clause 1; `shortreal`/`realtime`/assoc-of-string decls unsupported on POST (pre-existing E2002/E3010).
d/d06b_struct_member_e.sv (unpacked struct, string member E and int member E): POST A..D = verilator (1,2,0,1); E `us.k=-1` in `[-2:0]`: POST 1, verilator 0 -> see d06c.
d/d06c_struct_int_e.sv: POST `m=1 if-twin=1` = sv2v->iverilog `m=1 if-twin=1`; verilator `m=0 if-twin=1` (self-contradiction). Oracle note: verilator 5.052's case-inside compares an int-wide range [-2:0] unsigned too (also d/d04b [-2:1]); the P4 pin text says "narrow" and that `[-2:-1]` works — `[-2:-1]` gives the same answer unsigned, so it is not evidence. NON-BLOCKING doc correction (O1).

### Q1b name-keyed rewrites over labels (block-local flatten, function locals, param override)
design d/d07_shadow.sv: block-local `lo`/`v` shadowing module `lo`/`v` in a range bound, a value item and E; function-local `lo` shadow; `sub #(.LO(4),.HI(6))` / `#(.LO(8),.HI(9))` params in an always_comb case-inside range.
POST `m1=1 m2=1 m3=1 m4=1 ms1=1 ms2=0 lo=9 v=9` = sv2v->iverilog = verilator. No finding.

### Q2c evaluation order E-before-items; string literals under a packed E
design d/d08_order_strlit.sv: E = writer call that changes the item var / range bound; `"b"`, `"ab"` items under bit[7:0]/bit[15:0] E.
POST A m=1 cnt=5, B m=1 lo=10, C 1, D 1, E 2 = sv2v->iverilog = verilator. No finding.

### Q4b byte identity of the plain-case lanes (case_ctx_operator extraction, hoist visibility, lower_case early return)
code: stmt_flow.rs diff = verbatim move of the operator match into `case_ctx_operator` + `fn`->`pub(crate) fn` + an early return keyed on `CaseKind::Inside`.
design d/d09b_plain_regress.sv (§12.5 ctx operator E, `(min:typ:max)` E, casez, casex, fill item, string E, call E capture, operator item width): PRE stdout == POST stdout byte-identical (diff empty); values p1 1, p2 0 (= known P6), p3 1, p4 1, p5 2, p6 1, p7 3 cnt=1, p9 2.
### Q6 wildcard arms on CaseKind
grep `CaseKind::` (non-test): only expr_special.rs:895/911-923 (explicit arms; `Inside` = loud internal error) and stmt_flow.rs:636 (`matches!`); parser assertions.rs:489-491 maps keywords. No `_ =>` swallows `Inside`.

## consumer census (producer census of labels)
| consumer (file:line) | sees Inside? | behaviour | measured |
|---|---|---|---|
| parse_case -> parse_case_inside (hdl-parser stmt_ctl.rs:97,146) | producer (only one) | labels InsideEq($,v) / LogAnd(Ge($,lo),Le($,hi)) | d03, d10 (F1) |
| parse_unique_priority (assertions.rs:494) | kind-blind | appends Default `$__vita_unique_violation` | diff lens d09 |
| monomorph subst/rename (monomorph.rs:207,668) | in place, kind untouched | renames idents in labels; `$` has no name | d07 (param override; parameterized-class path not run: UNVERIFIED) |
| AST call hoist (hoist/general_stmt.rs:118, hoist/special.rs:531) | kind copied, items cloned | hoists the scrutinee only | d04, d04b, d08 |
| sva_decl rewrite_sampled (sva_decl.rs:279) | kind copied | maps each label | not run (UNVERIFIED) |
| lower_stmt (stmt_main.rs:777) -> lower_case (stmt_flow.rs:636) | yes | the only lowering; early return to lower_case_inside | all designs |
| case_cmp (expr_special.rs:894) | Inside arm = loud internal | string lane passes `Case` | d06b |
| frames_reserve collect_case_spans (frames_reserve.rs:229,647,709) | kind-blind | capture slot by span | d05d, d07 |
| definite assignment (da/mod.rs:488, da/reads.rs, da/writes.rs, da/loops.rs:54) | kind-blind | label reads walked; completeness = has_default | d07 |
| block-local flatten (block_local/mod.rs:57,130; block_local/hoist.rs:819; block_local_class.rs:713) | kind-blind | renames in labels | d07 |
| frames_classify expr_reads_only_locals (frames_classify.rs:21,114,240,836,859) | kind-blind | `Dollar => true` (P stands for E, walked separately) | d05d |
| collect_callee_stmt (ast_query.rs:141) | kind-blind | call names in labels | — |
| package purity (package.rs:115 -> pkg_expr_pure_with `_ => false` on Dollar) | yes | `p::f()` loud E3009 | implementer L17 (not re-run) |
| frames_classify_write.rs:277,348; pkg_scoped_frames.rs:363; multidriver.rs:108,202; ports.rs:132; hier.rs:36,76; proc_builder.rs:145; hoist/mod.rs:460; const_level_header.rs:368 | kind-blind walks (bodies / reads) | no label evaluation | — |
| const_fn interpreter (no Stmt::Case arm) | refuses | E3009 (plain case too) | implementer m5 |
| GenItem::Case consumers (generate.rs, toplevel.rs, expr_size_hier.rs, cond_names.rs, gen_enum.rs, md_return.rs, decl_collide.rs) | no (generate case inside = E2002) | — | — |
| cli / sim-ir / sim-engine | no Stmt::Case consumer (grep) | — | — |
| `.vu` postcard + SchemaHash | yes | old .vu -> E9002 rc=2 both directions; POST round trip runs | vu/ |

## findings
F1 NON-BLOCKING (pre-existing root, new observable flip) — hdl-parser/src/stmt_ctl.rs:97 `if self.at_ident_kw("inside")` takes the case-inside branch for an identifier `inside` that the design declared. vita lexes `inside` as an identifier (PRE accepts `reg [3:0] inside;`), so a design that is legal 1364-2005 / illegal 1800 flips meaning silently.
  d/d03_v2005_inside_ident.v: PRE `A m=1`, `B m=1` · iverilog 13 default generation `A m=1`, `B m=1` · POST `A m=0`, `B m=0` rc=0, no diagnostic. d/d10 (`casez (x) inside[1:0]:`): PRE m=1 = iverilog; POST E2002 (loud).
  Why not BLOCKING: vita applies 1800 keyword rules everywhere else (d/d03b: `reg [3:0] logic;`/`bit` -> E2002 on PRE), so the source is illegal under vita's language and PRE's value was a silent accept, not a correct result; POST's reading of the case statement is the 1800 reading. Census row m9 is value-coincident (v=1 vs inside[1:0]=2: m=0 in both readings) and its "iverilog rejects" holds only for -g2012.
  fix shape: refuse a declaration (and port/member name) spelled `inside` with the reserved-keyword E2002 the lexer gives `logic`/`bit`; then `case (e) inside` has one parse. Alternative (narrower): the parser records declarations named `inside` and the case-inside branch errors when one exists.
O1 NON-BLOCKING (oracle record) — verilator 5.052 case-inside compares an int-wide range that spans zero unsigned: d/d06c `us.k=-1` in `[-2:0]` verilator m=0 while its own `us.k inside {[-2:0]}` = 1; d/d04b `[-2:1]` verilator 0, sv2v 1, POST 1. The P4 pin says "narrow" and cites `[-2:-1]` working — that range gives the same answer unsigned, so it is not evidence. Correct the P4 text.
## pre-existing (PRE also has it)
- F1 root: `inside` accepted as an identifier (d/d03 PRE).
- P6: `case ((1:a + b:2)) 9'h100` PRE = POST 0 (d/d09b p2); oracle value from the implementer's cell, not re-measured here (my sv2v run aborted in iverilog on the design's string case).
- P3: `if (f(-1) inside {[-2:1]})` runs f twice on POST (d/d04 cnt 3->5, verilator +1); PRE not run on that line alone (UNVERIFIED on PRE; documented by the implementer as the parse_inside clone).
- E9002 for a stale `.vu` says "rerun `velab`" (vu/b.log) — the stale artifact comes from vcmp.

# Round 2 (delta: F1 fix — parser `inside` branch narrowed + `name_inside_binds` at lowering)
status: DONE — verdict FINDINGS (1 BLOCKING: F2), product_shakes: yes
POST2 = S/post2/vita md5 f491ab7b4921b912baa27b9828641232
## R2 Q1 name census, batch 1 (r2/)
- r2a_1364.v (1364-legal; port, genvar, generate net, real/string/wide param, specparam, function declared after use, function in generate, function formal, task local): PRE 11×`m=1` = iverilog default gen 11×`m=1`; POST2 11× E3009 (name reason). No gap.
- r2b_pkg.sv (SV-only: wildcard/explicit import of package var and function, package localparam, package enum label): PRE 6×`m=1`; POST2 6× E3009. No gap.
- r2c_cls_let.sv (module `let inside`, class property `inside` in the method's own class): PRE 2×`m=1`; POST2 2× E3009. No gap.
- r2d_unit.sv (`$unit` variable): PRE and POST2 E2002 (vita has no `$unit` declarations). No route.
## R2 Q1 batch 2 + Q2 ordering (r2/)
- r2e_order.v: module var `inside` declared after the use (module scope, and after a generate block holding the use): PRE E3010 "used before it is declared" ×2; iverilog default gen rejects (declaration after use); POST2 E3009 ×2. Task input formal, automatic recursive function formal: POST2 E3009 ×2. No gap.
- r2g_sv.sv (SV-only): interface-internal var, property and method inherited from a base class, package routine body over a package var (`pk::pf`), wildcard import placed after the use: PRE 5×`m=1`; POST2 E3009 on all 5 (plus the package-purity E3009 for `pk::pf`). No gap.
- r2i_implicit.v: implicit 1-bit net `inside` from `assign inside = 1'b1` (before/after the use) and from a port connection: PRE 3×`m=1` (W2003 implicit-net warnings at parse); POST2 E3009 ×3 (vita registers implicit nets at parse time, before lowering). No gap.
- r2f_upward.v / r2h_upward_gen.v: function `inside` declared in the PARENT module (r2f) or in the parent's generate block around the instance (r2h), called by simple name in the child's label `inside(3)`: FINDING F2.
## R2 Q3 re-run of every round-1 design (POST vs POST2, stdout+stderr+rc, r2/rerun/)
18 designs without an `inside` declaration: byte-identical. d03 (declares `inside`): POST `A m=0 B m=0` rc=0 -> POST2 E3009 ×2 (the F1 fix). d10 (casez over declared `inside`): POST E2002 -> POST2 `F casez x=2 vs inside[1:0] m=1` = PRE = iverilog.

## R2 findings
F2 BLOCKING — same root class as round-1 F1 (the parser's `inside` branch over a name `inside` that the design declares), new instance: IEEE upward search for a function by simple name across module boundaries. vita's resolvers do not implement it, so `name_inside_binds` (case_inside.rs, built only from vita's own lookups) answers "free". That fallback reads as a fact, and the statement is accepted.
  r2f_upward.v (`module top; function [3:0] inside; input [3:0] a; inside = a + 1; endfunction child c(); …` + child `case (x) inside(3): m = 1; default: m = 0;` x=4):
    PRE `error[VITA-E3010] … call to undeclared function `inside` [in top.c]` rc=1 · POST2 `child upward m=0` rc=0 (silent) · iverilog 13 default gen `child upward m=1` · verilator 5.052 `--default-language 1364-2005`: `%Error: … Can't find definition of task/function: 'inside'` (rejects: not evidence).
  r2h_upward_gen.v (function inside the parent's `generate if (1) begin : g … child2 c(); end`, label `inside(3) + 4'd0, 4'd9:`): PRE E3010 · POST2 `child2 upward-gen m=0` rc=0 · iverilog `m=1` · verilator rejects.
  Loud on PRE, silent and different from the one accepting oracle on POST2. (POST round 1 has the same: it had no check.) The IEEE rule (1364-2005 §12.7 / 1800-2017 §23.9: a task/function name search continues into higher-level modules) is quoted from memory: UNVERIFIED here; the finding rests on the measured loud->silent change.
  fix shape: do not depend on vita's scope model for this refusal. Record at parse time whether ANY declaration in the compilation (variables, nets, ports, params, genvars, functions, tasks, blocks, typedefs, enum labels, class members, lets, implicit nets) is spelled `inside`, and refuse `case … inside` design-wide when it is. A narrower alternative: `name_inside_binds` also asks a design-wide routine-name set (every module's/generate scope's functions) for `inside`. The narrower one leaves every other IEEE route vita does not model open.
## R2 pre-existing
- vita has no upward function search (r2f/r2h PRE E3010) — the root that F2 turns from loud into silent.
- PRE runs `m_assign_after` (implicit net used before the `assign` that declares it) where iverilog rejects it (r2i): laxity, loud on POST2 by the name refusal.
- Known residue kept: `case (x) inside == 4'd5:` over a declared `inside` = E2002 (loud).

# Round 3 (delta: name_inside_binds reverted; design-wide token-count decline `TopItem::InsideNameUse`)
status: DONE — verdict CLEAN on soundness (0 BLOCKING; 1 NON-BLOCKING diagnostic note O2), product_shakes: no
POST3 = S/post3/vita md5 9b0c1623e3acb71130efe3acb7713a89
## R3 Q1 TopItem consumer census (code read)
TopItem:: sites (non-test): cli frontend.rs (timescale module filter, `_ => None`), pipeline.rs + staged.rs (compose: nameless items incl. InsideNameUse pushed into the merged unit from every composed CU), elaborate classes.rs:113 `_ => {}`, driver.rs:650/666 `_ => None`, md_return.rs (explicit inert arm), struct_arm.rs:140 `_ => false`, sva_clocking.rs:20 / toplevel.rs:163,301-303 (`if let Module` / `_ => continue`), hdl-parser params.rs:43,144 `_ => {}`, module_items.rs (producer). api.rs:32 `items.is_empty() && errors.is_empty()` -> None: a marker-only unit needs a stray `inside` token outside any module, which is already a parse error. No wildcard arm gives the marker a meaning.
## R3 Q2 rewinds (code read)
`self.pos =` sites: type_params.rs:473,487,499 (speculative packed type), 905,912,921 (type-parameter override), expr_primary.rs:216 (`$bits(<type>)`), functask.rs:417 (`const ref`, no expression parsed). The parser never truncates `errors`. The operator site (expr.rs:195) pushes only when an lhs is followed by `inside`, and `parse_inside` then needs `{`. A speculative push over a NAME therefore either errors (kept, loud) or sits in `<expr> inside {…}`, which the re-parse also reads as the operator. No over-subtraction path found.
## R3 Q3 entries (code read)
One `Elaborator::new` (elaborate api.rs:147) and one `run` call (api.rs:154); every `elaborate_*` API reaches it. `run` sets `inside_name_use` as its first statement (driver.rs:648); `lower_case_inside` refuses at entry (case_inside.rs:102).
## R3 Q2 count completeness (measured, r3/)
- r3a_macro.v: the only name uses come from a macro (`define INS inside; `reg [3:0] `INS;`): PRE `r3a macro-name m=1` = iverilog; POST2 E3009 (scope); POST3 E3009 design-wide "(first at r3a_macro.v:3)".
- r3b_marg.v: name supplied as a macro argument (`DECL(inside)`, `SET(inside, …)`): PRE m=1 = iverilog; POST3 E3009 "(first at r3b_marg.v:4)".
- r3c_inc.v + r3c_inc.vh: declaration in an `include`d file: PRE m=1 = iverilog; POST3 E3009 "(first at r3c_inc.vh:1)".
=> the counted vector is the post-preprocess stream, and macro bodies, macro args and includes count.
- parser `"inside"` sites: only expr.rs:191 (operator) and stmt_ctl.rs:104 (case-inside); both push (expr.rs:195, stmt_ctl.rs:157). No third keyword site that would count a keyword as a name.
## R3 Q3 entries (measured)
- r3d_xa.v (top: `function [3:0] inside`) + r3d_xb.v (child: `case (x) inside(3)`), the F2 shape split over two files. One-shot: PRE E3010 · POST2 `r3d child m=0` rc=0 · POST3 E3009 "(first at r3d_xa.v:2)" · iverilog `r3d child m=1`.
- staged, work library (`vcmp --work lib=wl` per file, `velab -L lib=wl --top top`): POST3 velab rc=1, E3009 design-wide "(first at byte 29)". Per-file `.vu` pair -> `velab` rejects two inputs (rc=3, "expected exactly one .vu input"; composition goes through -L).
- `.vu`: PRE `.vu` and POST2 `.vu` -> POST3 velab: E9002 schema mismatch (loud stale). POST3 vcmp->velab->vrun of vu/ci.sv `m=1` = one-shot.
## R3 Q4 re-run POST2 vs POST3 (stdout+stderr+rc, r3/rerun/)
Round-1: 17 designs without a name `inside`, plus d10 (casez over a declared `inside`, no case-inside), d03b, vu plain/ci: byte-identical. d03: loud->loud (2 E3009, now design-wide).
Round-2: r2a/r2b/r2c/r2e/r2i/r2g: loud->loud (same error counts). r2f `child upward m=0` -> E3009, r2h `child2 upward-gen m=0` -> E3009 (silent->loud: F2 closed). r2d E2002 identical.
No change outside designs that use `inside` as a name; every change is silent->loud or loud->loud.
## R3 findings
O2 NON-BLOCKING, new instance (diagnostic only, not F1's class): in the staged library flow the decline names its first use as "first at byte 29" with no file (no span resolver at velab). With several compilation units a bare byte offset does not identify the file. The refusal itself is right and loud.
Residue accepted by the slice (unchanged): SV-legal escaped `\inside` beside a case-inside is now refused (t/esc2; PRE was E3010 too); a `begin_keywords "1364-2005"` region that declares `inside` refuses every SV case-inside in the design (PRE was E2002 for those); `inside == …` / `inside.v` / `inside'(…)` after `case (x)` = E2002.
