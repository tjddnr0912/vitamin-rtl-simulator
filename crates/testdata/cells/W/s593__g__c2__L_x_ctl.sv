`timescale 1ns/1ns
module sub #(parameter type T = logic signed [7:0]) ();
  localparam T X = -8'sd4;
  initial $display("cmp=%0d mix=%0d", X > -5, X + 1'b1);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
