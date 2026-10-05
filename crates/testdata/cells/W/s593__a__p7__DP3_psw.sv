`timescale 1ns/1ns
module sub;
  parameter logic signed [63:0] P = 0;
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: (P ==? 4'sb1?00)+3];
  initial #1 $display("ps=%b", ps);
endmodule
module t;
  sub u();
  defparam u.P = -4;
  initial #5 $finish;
endmodule
