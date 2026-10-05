`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit AB [0:1] = '{1'b1, 1'b0};
  initial #1 $display("RT=%0d", ((X + AB[0]) == 8'hFD));
  initial #20 $finish;
endmodule
