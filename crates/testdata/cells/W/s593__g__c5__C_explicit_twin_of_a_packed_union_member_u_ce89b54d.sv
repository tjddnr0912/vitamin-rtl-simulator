`timescale 1ns/1ns
module top;
  typedef union packed { logic signed [7:0] f; logic [7:0] g; } u_t;
  u_t s;
  logic signed [31:0] d;
  initial begin
    s.f = 8'hF0;
    d = s.f;
    $display("bits=%0d raw=%0d d=%0d neg=%0d", $bits(s.f), s.f, d, (s.f - 1) < 0);
  end
  initial #10 $finish;
endmodule
