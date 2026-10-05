`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -4;
  typedef struct packed { logic [X+8:0] a; } s_t;
  s_t s;
  initial $display("sb=%0d", $bits(s));
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
