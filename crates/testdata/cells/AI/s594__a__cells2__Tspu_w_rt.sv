`timescale 1ns/1ns
module t;
  typedef struct packed { logic [3:0] hi; logic [3:0] lo; } u_t;
  localparam u_t TA [0:1] = '{8'hF0, 8'h12};
  initial #1 $display("RT=%0d", ((TA[0] + 1'b0) == 8'hF0));
  initial #40 $finish;
endmodule
