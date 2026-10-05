`timescale 1ns/1ns
module t;
  function automatic logic signed [3:0] fxs(input int a); logic signed [3:0] t; return t - 4'sd4; endfunction
  localparam logic signed [3:0] PA = fxs(2);
  localparam L = (PA ==? 4'sb1?00);
  initial $display("L=%b", L);
  initial #5 $finish;
endmodule
