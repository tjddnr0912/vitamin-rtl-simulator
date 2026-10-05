`timescale 1ns/1ns
module sub #(parameter bit [3:0] X = '0);
  initial $display("X=%b", X);
endmodule
module top;
  sub #(.X(4'bx01z)) u();
  initial #100 $finish;
endmodule
