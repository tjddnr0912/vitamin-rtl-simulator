`timescale 1ns/1ns
module t;
  function automatic logic signed [3:0] fxs(input int a); logic signed [3:0] t; return t; endfunction
  if ((fxs(2) - 4'sd4) ==? 4'sb1?00) begin : gi initial #1 $display("GI=then"); end else begin : gi initial #1 $display("GI=else"); end
  initial #5 $finish;
endmodule
