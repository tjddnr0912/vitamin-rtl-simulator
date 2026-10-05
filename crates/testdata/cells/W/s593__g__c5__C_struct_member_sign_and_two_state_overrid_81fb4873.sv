`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) ();
  typedef struct packed { T f; } s_t;
  s_t s;
  logic signed [31:0] d;
  initial begin
    s.f = 8'hF0;
    d = s.f;
    $display("bits=%0d raw=%0d d=%0d neg=%0d", $bits(s.f), s.f, d, (s.f - 1) < 0);
  end
  initial #10 $finish;
endmodule
module top; m #(.T(bit signed [7:0])) u(); endmodule
