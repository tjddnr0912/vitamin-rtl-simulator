`timescale 1ns/1ns
module m #(parameter int N = 1);
  localparam logic signed [7:0] X = -4;
  localparam L = ((X + {N{1'b0}}) == 8'hFC);
  initial #1 $display("L=%0d", L);
endmodule
module t;
  m #(.N(2)) u();
  initial #20 $finish;
endmodule
