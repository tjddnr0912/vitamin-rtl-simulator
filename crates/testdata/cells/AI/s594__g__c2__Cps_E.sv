`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  logic [15:0] v = 16'hFFFF;
  initial #1 $display("ps=%0d", $bits(v[0 +: (A[1][1:0] | 2'b11 + 2'd2)]));
  initial #20 $finish;
endmodule
