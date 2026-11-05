(§2 md-select-index-context · OPEN · in Ranges / bounds / selects) (an md packed parameter's select index is evaluated in the offset's context):
* symptom: the select arms of `hdl-parser/src/packed_md.rs` place the user's index inside `(i - lsb) * stride`, so it takes that tree's width and sign
* IEEE 1800-2017 §11.5.1 (ref :6661): `Indexing expressions are self-determined and treated as unsigned`
* witness: q30_idx_narrow `parameter logic [1:0][L3 - 5'sd14 : 0] P = 4'b1001;` `logic [1:0] k = 2'b11;` `P[k + 2'd1]`: PRE `A p=xx` = sv2v → iverilog
* verilator `A p=01` (W/qsymmix/plan2/cells)
* witness: q2a_idx_signed_signedstride `logic signed [1:0] k = 2'sb11;` `P[k]` over a signed stride: PRE `A p=xx`, verilator `A p=11`
* over an unsigned stride (q2b) PRE `A p=11`
* oracles: verilator and hand-IEEE (iverilog refuses a packed array parameter)
* status: OPEN
