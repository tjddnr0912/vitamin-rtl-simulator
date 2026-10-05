`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  logic d [(A[1][1:0] | 2'b11 + 2'd2):0];
  initial #1 $display("ds=%0d", $size(d));
  initial #20 $finish;
endmodule
