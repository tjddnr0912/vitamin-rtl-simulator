`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, 8'sd2};
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  localparam bit C = 1;
  localparam logic [15:0] W = 16'h00F4;
  logic [($clog2(A[0])) + 0 : 0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #20 $finish;
endmodule
