(§5.2 Q-TPCARRIER · rank ① F · source Q-DECLSELBOUND plan 2 (Q11, O2: p2 c01–c04, lens-sound t01 t10)) (type parameters are carried by parse-time stand-in names):
* slot: PARKED 2026-10-29 behind Q-WITHREAL, Q-PARAM2STATE (both halves), Q-LETCAST and Q-2STINVSEL, its door rows
* slot: after Q-LETCAST, the last of them on lane 2
* a type parameter is carried by stand-ins minted at parse (`T$w`, `T$s`, `T$d<i>a` / `b`, `T$p<i>a` / `b`, a package's `PT$w`: `hdl-parser/src/type_params.rs:168–169`, `:380–381`, `:944–962`, `type_param_packed.rs`, `pkg_type_param.rs`, `casts.rs:55`), and an overridable 1-D one registers `logic [T$w-1:0]` (§4.5.514), which records neither direction nor LSB, so a name it types reads the wrong bits and every consumer of a declared range (`elaborate/src/params.rs:846` `param_decl_range_opt`, read back by `param_sel_range` at `:952` from `const_select.rs:551`, `packed.rs:1716`, `const_array.rs:239`, `edge_gate.rs:1487`) reads the stand-in's range as if declared: `module t #(type T = logic [0:7]); localparam T VT = 8'h01; logic [(4'd9 / VT[7]):0] d;` iverilog = sv2v = verilator n=10, PRE n=1
* `[VT[0:3]:0]` with `VT = 8'hA5` all three n=11, PRE 1
* the value leaves the unit through an override (`c #(.P(VT[7])) u();` into a child with no type parameter: n=10, PRE 1)
* in every scope that parses `parameter type`: module and interface, a generate block (overridable in vita, `type_params.rs:170–172`, where IEEE 1800-2017 §6.20.1, ieee1800-2017.txt:5065–5067, makes a generate block's parameters localparams: §2 "Scoping" generate-param-overridable) and program (a package's, a `localparam type`'s and a body one's under a header keep the P5 stand-in: §2 "Ranges" non-overridable line; a routine- or block-local one is E2002 at parse: §3.a ⑤)
* §2 "An overridable 1-D `parameter type T = logic [8:1]`" is this row
* it re-derives on row 66's `TypeRec` (§4.5.631; both rewrite the parser's type registration, ER §3.6)
* PARKED 2026-10-29 (W/qtpcarrier/PLAN.txt, NO-GO): the 1-D carrier makes a T-typed read equal its plain twin's (scratch `W/qtpcarrier/scratch2.patch` c47d96fc: gate PASS, verilator 175 DIFF→SAME, 61 LOUD→SAME), so an open value defect reaching a 1-D packed name turns the stand-in's coincidental right reads wrong — its doors, each a prerequisite: D1 `parameter signed NS = 4'hF;` into `localparam T WT = NS;` (T = logic [0:7]) WT[0..3] (iverilog = sv2v = verilator) and D2 `parameter unsigned` WT[0] WT[2] (iverilog = verilator) — Q-PARAM2STATE's signing half
* D3 a 2-state parameter's invalid select (§11.5.1, hand-IEEE) — its 2-state half
* D4 `longint'(item)` sums (verilator) — Q-WITHREAL
* D5 a typed `let` formal (§11.12, hand-IEEE) — Q-LETCAST
* D6 a 2-state variable's invalid select (§11.5.1, verilator) — Q-2STINVSEL (W/qtpcarrier/plan/c, k, q; PRE `W/accept/vita-01cac4b3` 56e6bdf2)
* no walk-guaranteed exclusion reaches D3 (a design-global one closes D1 / D2 only), so none is built
* un-park: the four rows landed and the door census on the new PRE (PLAN §5 B0) shows 0 right→wrong reads (a new door is a new prerequisite)
* expected then: 215 cells wrong→right, 61 loud→right, 0 new loud
* PRE 9d60771d
* note: Q-VALRULES fix (d) slice A (§4.5.636) moved the stand-in's width rule into `crates/hdl-ast/src/range_rules.rs` verbatim (`sym_typedef_width`, `:817`, its `[X-1:0] -> X` shortcut at `:819`, called from `hdl-parser/src/type_params.rs:634`), so the carrier keeps PRE's count spelling and stays a second count → range home (ER §5.1), this row's (W/qvalrulesd/plan2/PLAN.txt E3-b)
* note: fix (d)'s round 1 measured a carrier built from the true count: tp4 PRE `A l=-1 o=1 w=10` (= iverilog = sv2v = verilator) became `A l=1 o=x w=0x`, tpd the same (W/qvalrulesd/RULINGS-r1lens.txt F2)
* silent-wrong, pre-existing
* source: r2 lens-sound F1 (k01–k12, r09–r14, f01–f06), lens-diff F1 (g1–g17); Q-TPCARRIER plan (W/qtpcarrier/PLAN.txt, 2026-10-29)
