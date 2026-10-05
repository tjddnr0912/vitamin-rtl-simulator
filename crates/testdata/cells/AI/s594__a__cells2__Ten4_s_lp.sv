`timescale 1ns/1ns
module t;
  typedef enum logic [3:0] {E0 = 4'd0, E9 = 4'd9, E12 = 4'd12} e_t;
  localparam e_t TA [0:1] = '{E12, E9};
  localparam L = ((TA[0] + 1'sb0) < 0);
  initial #1 $display("L=%0d", L);
  initial #40 $finish;
endmodule
