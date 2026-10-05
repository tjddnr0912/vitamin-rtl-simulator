`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = 300;
  logic [X:0] v;
  initial $display("X=%0d vb=%0d", X, $bits(v));
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
