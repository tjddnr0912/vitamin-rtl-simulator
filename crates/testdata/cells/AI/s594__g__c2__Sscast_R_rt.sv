`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [15:0] W = 16'h00F0;
  localparam int N = 2;
  initial #1 $display("RT=%0d", $unsigned(X + {N{1'b0}}));
  initial #20 $finish;
endmodule
