`timescale 1ns/1ns
module sub;
  parameter logic signed [63:0] P = 0;
  logic [(P ==? 4'sb1?00)+3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
endmodule
module t;
  sub u();
  defparam u.P = -4;
  initial #5 $finish;
endmodule
