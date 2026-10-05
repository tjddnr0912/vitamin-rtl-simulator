`timescale 1ns/1ns
module t;
  typedef struct packed signed { logic [3:0] hi; logic [3:0] lo; } s_t;
  localparam s_t TA [0:1] = '{8'hF0, 8'h12};
  localparam L = ((TA[0] + 1'b0) == 8'hF0);
  initial #1 $display("L=%0d", L);
  initial #50 $finish;
endmodule
