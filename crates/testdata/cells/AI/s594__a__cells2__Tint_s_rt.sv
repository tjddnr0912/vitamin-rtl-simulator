`timescale 1ns/1ns
module t;
  localparam integer TA [0:1] = '{-5, 7};
  initial #1 $display("RT=%0d", ((TA[0] + 1'sb0) < 0));
  initial #40 $finish;
endmodule
