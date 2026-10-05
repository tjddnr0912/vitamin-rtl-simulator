`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  localparam int R = (X ==? 4'b1?00) ? 5 : 7;
  initial $display("R=%0d", R);
endmodule
module top;
  sub #(.T(logic signed [7:0])) u();
  initial #100 $finish;
endmodule
