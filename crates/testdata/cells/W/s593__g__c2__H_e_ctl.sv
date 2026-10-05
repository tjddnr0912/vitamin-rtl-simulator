`timescale 1ns/1ns
module sub #(parameter type T = logic signed [7:0], parameter T X = -8'sd4);
  logic [15:0] w16; int i32;
  initial begin w16 = X; i32 = X; $display("w16=%h i32=%0d", w16, i32); end
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
