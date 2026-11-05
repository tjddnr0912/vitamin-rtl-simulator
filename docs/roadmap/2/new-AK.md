(§2 🆕 AK · OPEN · in Size cast / signedness) (a non-ANSI `input signed` port redeclared as an unsigned net reads unsigned):
* symptom: a non-ANSI port declared `signed` whose net declaration has no `signed` reads unsigned at exit 0, where IEEE 1800-2017 §23.2.2.1 makes the net signed too
* witness: d1b_nansi_08 `module c(a); input signed [3:0] a; wire [3:0] a; initial #1 $display("NA d=%0d b=%b n=%0d lt=%0d", a, a, $bits(a), (a < 0)); endmodule` `module top; c u('1); endmodule`
  * PRE `NA d=15 b=1111 n=4 lt=0` at exit 0, iverilog and sv2v → iverilog `NA d=-1 b=1111 n=4 lt=1`
  * verilator `%Error: t.sv:2:16: Non-ANSI I/O declaration of signal conflicts with type declaration: 'a'`
* note: IEEE 1800-2017 §23.2.2.1 (ieee1800-2017.txt:32051-32052): "If either the port or the net/variable is declared as signed, then the other shall also be considered signed."
* note: and its example (ieee1800-2017.txt:32070): `wire [7:0] c; // net c inherits signed attribute from port`
* note: the reverse order is right: d1b_nansi_06 (`input [3:0] a; wire signed [3:0] a;`) PRE `d=-1 lt=1` = iverilog
* note: silent-wrong, pre-existing (PRE 92805863 = POST, Q-DECLTYPE S12 differential lens D-N1)
* oracles: iverilog, sv2v → iverilog and hand-IEEE (verilator refuses the source)
* status: OPEN
* source: W/qdecltype/lens1/diff/orc/d1b_nansi_08, W/qdecltype/lens1/diff/REPORT.txt N1
