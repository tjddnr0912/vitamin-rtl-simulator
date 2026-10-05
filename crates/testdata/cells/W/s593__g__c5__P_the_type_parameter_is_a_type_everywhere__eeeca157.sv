`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  typedef struct packed { T a; logic b; } s_t;
  s_t s;
  initial begin s = '1; #1 $display("D=%h %0d", s.a, $bits(s)); end
endmodule
module top;
  m u1(); m #(.T(logic [3:0])) u2();
  initial #5 $finish;
endmodule
