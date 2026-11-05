(§3.b const-x-real · OPEN · in VCD / real conversion) (a constant with x / z bits converted to real does not fold):
* symptom: a constant with an x/z bit converted to real is E3009, where the run-time conversion (§4.5.599) and iverilog read each x/z bit as 0: `localparam real P = 4'bx011;` ("value is not a constant: 4'bx011 has no constant-fold arm"), `#(.P(4'bx011))` onto a real parameter ("a real override cannot be folded"), `localparam int W = $rtoi(4'bx011 * 1.0);`, `localparam real P = 4'bx011; if (P > 2.0)`
* oracles: iverilog, verilator and sv2v → iverilog `P=3.000000`, `bits=4`, `gen=hit`
* cause: `const_real.rs` `const_selfdet_real` and the override's unknown-bit filter (`params.rs`) decline
* oracles: 3 oracles
* status: OPEN
