(§5.2 Q-LETCAST · rank ① · source Q-MEMBSRC PLAN4 design review (D4-3)) (a `let` formal's declared type is dropped at parse):
* slot: before Q-TPCARRIER and Q-MEMBSRC
* slot: no prerequisite: lane 2 may run it earlier while its next row waits
* T′, a `let` formal's declared type is dropped at parse (`hdl-parser/src/sva_seq.rs:140` `parse_sva_formals` and `:171` `parse_one_sva_formal` skip the type and keep the name; `LetDecl { formals: Vec<Ident> }`, `hdl-ast/src/lib.rs:481`) and elaborate substitutes the actual textually (`elaborate/src/sva_decl.rs` `lower_let_use` → `subst_expr`), where IEEE 1800-2017 §11.12 (ieee1800-2017.txt:12998–13001) casts the actual to the formal's type first: with `localparam logic [15:8] VA = 8'hA5; localparam logic [3:0] N = 4'd7;`, `let f(logic [7:0] x) = VA[~x];` `f(N)` reads `1` (IEEE x: ~8'd7 = 248 is outside [15:8]), `let inv(logic [7:0] x) = ~x;` `inv(N)` is `8` (IEEE `f8`) and `let wd(logic [7:0] x) = $bits(x);` `wd(N)` is 4 (IEEE 8)
* an untyped formal (`let g(x) = ~x;` `g(N)` = 8) is textual in IEEE too
* no tool oracle: iverilog 13.0 `sorry: let declarations (f) are not currently supported`, sv2v `Parse error`, verilator 5.052 `Unsupported: let typed ports` — hand-IEEE (W/row24/membsrc/park/cells/d43)
* through a member (`let f(logic [7:0] x) = P.a[~x];`) it is Q-MEMBSRC's design-review D4-3 (lt/l1, l8, l1p)
* typed sequence and property formals (§16.8.1, ieee1800-2017.txt:17366–17370) are E3010 today and stay
* the fix shape decides the format (a typed formal in `LetDecl` is an hdl-ast SchemaHash re-pin, one at a time; a cast synthesized by the parser has no source spelling for `logic [7:0]` or a 2-state vector)
* no corpus witness (no `let` in the OpenTitan or VeeR checkouts): in row 24's fix path
* a door into Q-TPCARRIER (W/qtpcarrier/plan/k/k03: `{inv(N)}` into a `T`-typed name, hand-IEEE §11.12 `8'hF8`)
* PRE 01cac4b3
* silent-wrong, pre-existing
