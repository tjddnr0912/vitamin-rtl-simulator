`timescale 1ns/1ns
module m #(parameter logic [127:0] P = 0);
  initial #1 $display("P=%h", P);
endmodule
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  m #(.P(X + {N{1'sb0}})) u();
  initial #40 $finish;
endmodule
