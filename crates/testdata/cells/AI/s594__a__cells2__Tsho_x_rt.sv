`timescale 1ns/1ns
module t;
  localparam shortint TA [0:1] = '{-16'sd3, 16'sd2};
  initial #1 $display("RT=%0d", ((TA[0] + 16'd4) == 16'd1));
  initial #40 $finish;
endmodule
