`timescale 1ns/1ns
interface ifc #(parameter type T = logic [3:0], parameter T X = '1) ();
  initial $display("X=%0d lt0=%0d b=%0d", X, X < 0, $bits(X));
endinterface
module top;
  ifc #(.T(logic signed [3:0])) i();
  initial #100 $finish;
endmodule
