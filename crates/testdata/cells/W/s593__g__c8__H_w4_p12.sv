`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0], parameter T X = '0);
  localparam R = (X ==? 4'sb1?00);
  localparam RN = (X !=? 4'sb1?00);
  initial $display("R=%0d RN=%0d", R, RN);
endmodule
module top;
  sub #(.T(logic signed [7:0]), .X(8'sd12)) u();
  initial #100 $finish;
endmodule
