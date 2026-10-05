`timescale 1ns/1ns
module t;
  localparam logic [127:0] A128 [0:1] = '{128'hF0, 128'd2};
  localparam logic signed [7:0] X = -4;
  localparam L = ((X | A128[1]) == 128'hFFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFF_FFFE);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
