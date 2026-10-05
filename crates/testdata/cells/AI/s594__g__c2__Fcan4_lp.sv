`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [3:0] A4 [0:1] = '{8'hFF, 4'h1};
  localparam L = ((A4[0] + 4'd1) == 4'd0);
  initial #1 $display("L=%0d", L);
  initial #20 $finish;
endmodule
