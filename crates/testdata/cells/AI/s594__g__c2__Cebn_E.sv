`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  logic [A[1] - 8'd3:0] eb;
  initial #1 $display("eb=%0d", $bits(eb));
  initial #20 $finish;
endmodule
