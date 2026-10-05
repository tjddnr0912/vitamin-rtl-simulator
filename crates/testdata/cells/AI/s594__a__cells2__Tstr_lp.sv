`timescale 1ns/1ns
module t;
  localparam string TA [0:1] = '{"ab", "c"};
  localparam L = (TA[0] == "ab");
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
