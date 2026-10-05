`timescale 1ns/1ns
module t;
  typedef enum {F0, F1, F2} f_t;
  localparam f_t TA [0:1] = '{F2, F1};
  initial #1 $display("RT=%0d", ((TA[1] - 2) < 0));
  initial #40 $finish;
endmodule
