`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};
  logic [((2 ** AS[0]) == 0) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #20 $finish;
endmodule
