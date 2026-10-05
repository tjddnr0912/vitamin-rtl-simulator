`timescale 1ns/1ns
interface ifc #(parameter P = 1);
  initial #1 $display("P=%0d lt0=%0d B=%0d", P, P < 0, $bits(P));
endinterface
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  ifc #(.P(X + {N{1'sb0}})) u();
  initial #40 $finish;
endmodule
