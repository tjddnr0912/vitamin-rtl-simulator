`timescale 1ns/1ns
module t;
  typedef enum {F0, F1, F2} f_t;
  localparam f_t TA [0:1] = '{F2, F1};
  if (((TA[0] - 3) < 0)) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #40 $finish;
endmodule
