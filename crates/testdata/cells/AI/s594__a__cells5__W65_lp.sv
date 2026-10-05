`timescale 1ns/1ns
module t;
  localparam logic [64:0] A65 [0:1] = '{65'h1_0000_0000_0000_0002, 65'd2};
  localparam L = ((A65[0] + 1'b0) == 65'h1_0000_0000_0000_0002);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
