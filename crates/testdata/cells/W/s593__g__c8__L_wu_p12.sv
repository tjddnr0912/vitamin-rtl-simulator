`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0]) ();
  localparam T X = 8'sd12;
  localparam R = (X ==? 4'b1?00);
  initial $display("R=%0d", R);
endmodule
module top;
  sub #(.T(logic signed [7:0])) u();
  initial #100 $finish;
endmodule
