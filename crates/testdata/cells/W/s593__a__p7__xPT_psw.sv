`timescale 1ns/1ns
module sub #(parameter type T = logic [3:0], parameter T PV = '0);
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: (PV ==? 4'sb1?00)+3];
  initial #1 $display("ps=%b", ps);
endmodule
module t;
  function automatic logic signed [63:0] fxs64(input int a); logic signed [63:0] t; return t - 64'sd4; endfunction
  sub #(.T(logic signed [63:0]), .PV(fxs64(2))) u();
  initial #5 $finish;
endmodule
