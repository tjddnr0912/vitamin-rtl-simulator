`timescale 1ns/1ns
module t;
  localparam int N = 2;
  localparam logic [3:0] L = '1 ^ {N{1'b0}};
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
