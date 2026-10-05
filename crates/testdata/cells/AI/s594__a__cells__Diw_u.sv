`timescale 1ns/1ns
interface ifc #(parameter logic [127:0] P = 0);
  initial #1 $display("P=%h", P);
endinterface
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  ifc #(.P(A[0])) u();
  initial #40 $finish;
endmodule
