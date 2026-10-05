`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0]) ();
  localparam T X = -64'sd4;
  localparam R = (X ==? 4'sb1?00);
  localparam RN = (X !=? 4'sb1?00);
  initial $display("R=%0d RN=%0d", R, RN);
endmodule
module top;
  sub #(.T(logic signed [63:0])) u();
  initial #100 $finish;
endmodule
