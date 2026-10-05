`timescale 1ns/1ns
module t;
  localparam bit TA [0:1] = '{1'b1, 1'b0};
  initial #1 $display("RT=%0d", ((TA[0] + 1'b1) == 1'b0));
  initial #40 $finish;
endmodule
