`timescale 1ns/1ns
module leaf #(parameter type T = logic [3:0]) ();
  localparam T X = '1;
  initial $display("X=%0d lt0=%0d b=%0d", X, X < 0, $bits(X));
endmodule
module mid #(parameter type T = logic [3:0]) ();
  leaf #(.T(T)) l();
endmodule
module top;
  mid #(.T(logic signed [3:0])) u();
  initial #100 $finish;
endmodule
