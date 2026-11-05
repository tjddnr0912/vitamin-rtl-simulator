(§3.b mdrv-assert-local · OPEN · in Constants / parameters) (a local in an assertion action block is flattened onto the module variable):
* symptom: a local declared in an assertion's action block is flattened onto the module variable by bare name: `always_comb x = a;` + `a1: assert property (@(posedge clk) 1'b0) else begin logic [7:0] x; x = '0; end` is a false E3001 (Rule B; verilator and sv2v → iverilog `A1 x=11 A0 x=22`), its `always @*` twin prints `A1 x=00` (verilator `x=11`), and beside `assign x = a;` an `always @(x)` sees `[1] chg x=5a` and a deferred action's `$display` of the local prints `L x=11` (verilator none / `L x=5a`)
* scope the action-block local, then pass `true` from Rules A and B (`declares_local_named`)
* a deferred action's local read by `$bits(x[1])` reads the module's `x` (procedural `assert final (0) else begin logic [3:0][7:0] x; … end`: 1, Verilator 8)
* the module-item form refuses a declaring action block since §4.5.606
* pins: pins in `cont_assign_variable_drivers.rs`
* oracles: 2 oracles
* status: OPEN
