`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0], parameter T X = '0);
  initial $display("X=%b", X);
endmodule
module top;
  sub #(.T(bit [3:0]), .X(4'bx01z)) u();
  initial #100 $finish;
endmodule
