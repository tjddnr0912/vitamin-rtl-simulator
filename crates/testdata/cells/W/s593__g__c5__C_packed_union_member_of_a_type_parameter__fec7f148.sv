`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) ();
  typedef union packed { T f; logic [7:0] g; } u_t;
  u_t s;
  initial $display("raw=%0d", s.f);
  initial #10 $finish;
endmodule
module top; m #(.T(logic signed [7:0])) u(); endmodule
