`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) ();
  typedef struct packed { T f; } s_t;
  s_t s;
  initial begin
    s.f = '1;
    $display("bits=%0d neg=%0d hx=%0h", $bits(s.f), (s.f - 1) < 0, s.f);
  end
  initial #10 $finish;
endmodule
module top; m #(.T(logic signed [79:0])) u(); endmodule
