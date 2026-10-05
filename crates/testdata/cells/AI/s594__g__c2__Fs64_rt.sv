`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam longint AQ [0:1] = '{-64'sd4, 64'sd2};
  initial #1 $display("RT=%0d", (((AQ[0] + 64'd0) > 64'd100) == 1'b1));
  initial #20 $finish;
endmodule
