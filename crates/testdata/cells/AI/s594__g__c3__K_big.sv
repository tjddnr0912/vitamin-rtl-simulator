`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int NB = 32'h4000_0000;
  localparam L = ((X + {NB{1'b0}}) == 8'hFC);
  initial #1 $display("L=%0d", L);
  initial #20 $finish;
endmodule
