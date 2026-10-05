`timescale 1ns/1ns
interface ifc #(parameter int P = 0);
  initial #1 $display("P=%0d", P);
endinterface
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  ifc #(.P(X + {N{1'b0}})) u();
  initial #40 $finish;
endmodule
