`timescale 1ns/1ns
interface ifc #(parameter P = 1);
  initial #1 $display("P=%0d lt0=%0d B=%0d", P, P < 0, $bits(P));
endinterface
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  ifc #(.P(A[0])) u();
  initial #40 $finish;
endmodule
