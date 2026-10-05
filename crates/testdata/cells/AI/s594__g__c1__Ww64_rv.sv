`timescale 1ns/1ns
module t;
  localparam longint X = -4;
  localparam int N = 2;
  initial #1 $display("RV=%0d", X + {N{1'b0}});
  initial #5 $finish;
endmodule
