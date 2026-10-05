`timescale 1ns/1ns
module t;
  localparam time TA [0:1] = '{64'd5, 64'd7};
  initial #1 $display("RT=%0d", ((TA[0] - 64'd6) > 64'd100));
  initial #40 $finish;
endmodule
