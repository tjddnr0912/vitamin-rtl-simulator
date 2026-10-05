module top;
`define V 1
`include "inc_k5.svh"
`undef V
`define V 2
`include "inc_k5.svh"
  initial #5 $finish;
endmodule
