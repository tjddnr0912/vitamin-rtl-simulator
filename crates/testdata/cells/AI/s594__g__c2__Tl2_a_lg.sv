`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  localparam L = ((X + 2'b00) > 8'd100);
  initial #1 $display("L=%0d", L);
  initial #20 $finish;
endmodule
