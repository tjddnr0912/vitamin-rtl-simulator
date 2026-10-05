`timescale 1ns/1ns
module t;
  localparam logic [64:0] A65 [0:1] = '{65'h1_0000_0000_0000_0002, 65'd2};
  localparam logic signed [7:0] X = -4;
  localparam L = ((X | A65[1]) == 65'h1_FFFF_FFFF_FFFF_FFFE);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
