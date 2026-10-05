`timescale 1ns/1ns
interface ifc #(parameter logic [127:0] P = 0);
  initial #1 $display("P=%h", P);
endinterface
module t;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};
  ifc #(AS[0]) u();
  initial #40 $finish;
endmodule
