`timescale 1ns/1ns
module sub;
  localparam bit [3:0] X = 4'bx01z;
  initial $display("X=%b", X);
endmodule
module top; sub u();
  initial #100 $finish;
endmodule
