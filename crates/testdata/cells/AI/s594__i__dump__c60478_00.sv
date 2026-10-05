`timescale 1ns/1ns
module t;
  typedef struct packed { logic [3:0] hi; logic [3:0] lo; } u_t;
  localparam u_t TA [0:1] = '{8'hF0, 8'h12};
  localparam L = ((TA[0] + 1'sb0) < 0);
  initial #1 $display("L=%0d", L);
  initial #50 $finish;
endmodule
