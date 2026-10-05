`timescale 1ns/1ns
module t;
  typedef struct packed signed { logic [3:0] hi; logic [3:0] lo; } s_t;
  localparam s_t TA [0:1] = '{8'hF0, 8'h12};
  initial #1 $display("RT=%0d", ((TA[0] >>> 4) == 8'hFF));
  initial #40 $finish;
endmodule
