`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) ();
  typedef struct packed { T f; } s_t;
  s_t s;
  initial $display("u=%b", s.f);
  initial #10 $finish;
endmodule
module top; m #(.T(bit [7:0])) u(); endmodule
