`timescale 1ns/1ns
module sub;
  parameter logic signed [63:0] P = 0;
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: (P ==? 4'sb1?00)+3];
  initial #1 $display("ps=%b", ps);
endmodule
module t;
  function automatic logic signed [63:0] fxs64(input int a); logic signed [63:0] t; return t - 64'sd4; endfunction
  sub u();
  defparam u.P = fxs64(2);
  initial #5 $finish;
endmodule
