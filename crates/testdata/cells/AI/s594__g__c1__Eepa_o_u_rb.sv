`timescale 1ns/1ns
module t;
  localparam logic [7:0] X = 8'hFC;
  localparam bit C = 1;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  logic [((X | A[1][3:0]) == 8'hFE) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
