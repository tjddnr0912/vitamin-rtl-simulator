`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) ();
  typedef struct packed { T a; } s_t;
  s_t s;
  initial begin s.a = -1; $display("mem=%0d", s.a); end
endmodule
module top; m #(.T(logic signed [7:0])) u (); initial #10 $finish; endmodule
