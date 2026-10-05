`timescale 1ns/1ns
module t;
  typedef enum {F0, F1, F2} f_t;
  localparam f_t TA [0:1] = '{F2, F1};
  localparam L = ((TA[0] + 1'b0) == 32'd2);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
