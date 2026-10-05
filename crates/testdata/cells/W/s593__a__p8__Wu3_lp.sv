`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam T X = -4;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  localparam int N = 2;
  localparam bit C = 1;
  localparam L = ({N{X}} ==? 16'b1111_1100_1111_1?00);
  initial #1 $display("L=%b", L);
endmodule
module t;
  m #(.T(logic signed [7:0])) u();
  initial #5 $finish;
endmodule
