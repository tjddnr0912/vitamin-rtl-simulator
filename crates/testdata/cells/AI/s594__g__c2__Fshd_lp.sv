`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  if (1) begin : g
    localparam int A = 5;
    localparam L = ((X | A[1]) == 8'hFC);
    initial #1 $display("L=%0d", L);
  end
  initial #20 $finish;
endmodule
