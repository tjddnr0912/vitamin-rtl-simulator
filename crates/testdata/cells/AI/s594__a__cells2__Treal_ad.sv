`timescale 1ns/1ns
module t;
  localparam real TA [0:1] = '{1.5, 2.5};
  localparam real R = TA[1] + 1;
  initial #1 $display("R=%0.2f", R);
  initial #40 $finish;
endmodule
