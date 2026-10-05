`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  logic [15:0] v = 16'hABCD;
  initial #1 $display("pw=%h", v[0 +: ((A[1] - 8'd2) + 2'd3)]);
  initial #40 $finish;
endmodule
