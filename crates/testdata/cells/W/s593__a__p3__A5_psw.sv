`timescale 1ns/1ns
module t;
  function automatic logic signed [3:0] fxs(input int a); logic signed [3:0] t; return t - 4'sd4; endfunction
  localparam logic signed [3:0] PA = fxs(2);
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: (PA ==? 4'sb1?00)+3];
  initial #1 $display("ps=%b", ps);
  initial #5 $finish;
endmodule
