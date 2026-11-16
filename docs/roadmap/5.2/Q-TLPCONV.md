(§5.2 Q-TLPCONV · rank ① · source W/r68/plan/PLAN.txt §6 O-1 (owner O-1, 2026-11-15)) (the parser stores a typed localparam of unfolded width unconverted):
* slot: 29 · L2, after Q-TPCARRIER (owner O-1, 2026-11-15: a new row for the conversion, after Q-TPCARRIER)
* closes §2 typed-localparam-unconverted: neither row 68 nor row 48 converts it (that entry's line 3; row 48 "delivered 2026-10-29, no build", 48.md:2)
* the producer is W1 (`hdl-parser/src/params.rs:891-947`), the parser table's one value writer: it converts at the declared width only when that width is known, `let ctx_v = (typed && w > 0)` (`:911`)
* at w = 0 it stores the initializer unconverted: `self.try_const_index(&p.value)` (`:934`), filtered by `const_param_fits`, then `.insert(p.name.name.clone(), ConstEntry { v, w, signed, lane });` (`:940`), its `lane` WidthUnknown for a typed declaration (`:924-927`)
* witness: §2 typed-localparam-unconverted's c1_tp_mod, c1_tp_width and c1_twin_mod
* order (PLAN.txt O-1): storing no value needs a receiver that answers right for every reader of the table, and three do not exist yet, each measured on PRE spellings (W/r68/plan/m1, m2):
  * the type-parameter stand-in keeps no LSB (§5.2 Q-TPCARRIER): pfv_tp `A b=4 lo=0 L=2`, iverilog and verilator `A b=4 lo=2 L=2`
  * enum labels, the member-select remap and generate indices have no symbolic lane: pfv_enum and pfv_dbaser right → loud
  * a package has no defer marker to export, and its typedef twin leaves the name bare: pkx_twin right → loud, pkx_twin_decoy right → wrong
* note: PARSE-1's provenance piece (§5.2 PARSE-1, outs O7 and O8) would give the third receiver its origin mark (structure review 2, W/review/structure-2026-11-16/REPORT.txt Q3 B, Q4: a design claim, not run)
* note (2026-11-16, §4.5.645): the piece is built (§5.2 PARSE-1): this row is `lane`'s first reader by name (`#[allow(dead_code)] // first reader: 68 (b) / Q-TLPCONV`); W1's WidthUnknown lane holds the typed entries stored at w = 0, the ones this row converts
  * a read of `origin` here follows PARSE-1's consumer rule: decline on `!Source`, never trust on `Source` alone, re-census the construction sites of the variant trusted
* note: Q-CONSTSELLSB's re-entry RE-1 waits on this row (§5.2 Q-CONSTSELLSB)
* status: silent-wrong, pre-existing
* source: W/r68/plan/PLAN.txt §5.4, §5.5, §6 O-1; W/r68/ground/REPORT.txt G3
