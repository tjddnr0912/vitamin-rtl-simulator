`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  logic [(A[1][1:0] | 2'b11 + 2'd2):0] b;
  initial #1 $display("bb=%0d", $bits(b));
  initial #20 $finish;
endmodule
