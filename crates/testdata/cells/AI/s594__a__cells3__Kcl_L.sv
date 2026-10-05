`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam L = $clog2(X + 2'b00);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
