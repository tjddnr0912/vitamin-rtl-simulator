`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [15:0] W = 16'h00F0;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  localparam L = ((8'hFF + 8'd1 + (A[1] - 8'd2)) ? 5 : 7);
  initial #1 $display("L=%0d", L);
  initial #20 $finish;
endmodule
