`timescale 1ns/1ns
module m #(parameter P = 1);
  initial #1 $display("P=%0d lt0=%0d B=%0d", P, P < 0, $bits(P));
endmodule
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  m #(.P(X + {N{1'b0}})) u();
  initial #40 $finish;
endmodule
