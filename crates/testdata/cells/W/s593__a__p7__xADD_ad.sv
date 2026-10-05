`timescale 1ns/1ns
module t;
  function automatic logic signed [3:0] fxs(input int a); logic signed [3:0] t; return t; endfunction
  logic [3:0] arr [0:((fxs(2) - 4'sd4) ==? 4'sb1?00)+2];
  initial #1 $display("asz=%0d", $size(arr));
  initial #5 $finish;
endmodule
