`timescale 1ns/1ns
module sub;
  parameter logic signed [63:0] P = 0;
  localparam L = (P ==? 4'sb1?00);
  initial #1 $display("L=%b", L);
endmodule
module t;
  sub u();
  defparam u.P = -4;
  initial #5 $finish;
endmodule
