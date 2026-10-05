`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  logic [15:0] v = 16'hA5C3;
  initial $display("s1=%h s2=%b", v[X+7 -: 4], v[X+5]);
endmodule
module top;
  sub #(.T(logic signed [7:0])) u();
  initial #100 $finish;
endmodule
