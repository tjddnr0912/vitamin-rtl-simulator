`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0], parameter T X = '0);
  localparam R = (X ==? 4'b1?00);
  initial $display("R=%0d", R);
endmodule
module top;
  sub #(.T(int), .X(-4)) u();
  initial #100 $finish;
endmodule
