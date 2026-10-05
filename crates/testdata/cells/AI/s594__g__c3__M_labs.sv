`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};
  case (-8'sd4) AS[0]: begin : gk initial #1 $display("GC=item"); end default: begin : gd initial #1 $display("GC=def"); end endcase
  initial #20 $finish;
endmodule
