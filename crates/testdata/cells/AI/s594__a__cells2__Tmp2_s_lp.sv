`timescale 1ns/1ns
module t;
  localparam logic [1:0][3:0] TA [0:1] = '{8'hF0, 8'h12};
  localparam L = ((TA[0] + 1'sb0) < 0);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
