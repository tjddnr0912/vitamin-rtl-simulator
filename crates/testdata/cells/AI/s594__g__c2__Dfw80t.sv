`timescale 1ns/1ns
module t;
  localparam int N = 2;
  localparam logic [79:0] L = '1 ^ {N{40'h0}};
  initial #1 $display("L=%h", L);
  initial #20 $finish;
endmodule
