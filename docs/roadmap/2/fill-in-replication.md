(§2 fill-in-replication · OPEN · in Ranges / bounds / selects) (a fill in a constant replication's value list declines the concat):
* A fill in a constant replication's value list declines the concatenation: `localparam int I = 3; localparam logic [15:0] L = { 4'hA, {(4-I){'1}}, 8'h5C };` is E3009 "the concatenation `{…}` has no constant-fold arm", iverilog 13.0 = sv2v = verilator 5.052 `A v=155c` (b_fill_n; b_fill_z the zero-count twin, `0a5c`)
* loud (row 40 plan, W/row40/probe/b2)
* OPEN
