`timescale 1ns/1ns
module sub #(parameter type T = bit [3:0]) ();
  localparam T X = 4'bx01z;
  initial $display("X=%b", X);
endmodule
module top;
  sub #(.T(logic [3:0])) u();
  initial #100 $finish;
endmodule
