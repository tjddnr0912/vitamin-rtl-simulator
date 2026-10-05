`timescale 1ns/1ns
module t;
  function automatic logic [3:0] fx(input int a); logic [3:0] t; return t; endfunction
  logic [3:0] arr [0:(fx(2) ==? (4'b0?00))+2];
  initial $display("asz=%0d", $size(arr));
  initial #5 $finish;
endmodule
