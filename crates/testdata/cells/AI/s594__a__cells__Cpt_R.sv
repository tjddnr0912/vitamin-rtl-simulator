`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  initial begin #((X + {N{1'b0}}) * 1ns); $display("fired t=%0t", $time); end
  initial #400 $finish;
endmodule
