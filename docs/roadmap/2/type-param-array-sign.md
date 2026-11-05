(§2 type-param-array-sign · OPEN · in Size cast / signedness) (a constant array typed by a type parameter keeps the default type's sign):
* A constant array typed by a type parameter keeps the default type's sign per instance: `localparam T AT [0:1] = '{8'hFC, 8'hFC};` under `parameter type T = logic signed [7:0]`, `localparam int L = (AT[0] + 8'sd0) < 0;` in `sub #(.T(logic [7:0])) u2` gives `t.u2 L=1`, verilator `t.u2 L=0` (E1, §4.5.594; u1 and `logic signed [15:0]` u3 right; iverilog `sorry: unpacked array parameters are not supported yet.`; sv2v reads every element unsigned, u1 `L=0`)
* 🆕 AI's element arm reads that type (PRE = §4.5.594's cut)
* site not traced
* verilator + hand-IEEE (§6.20.3)
* OPEN
