`timescale 1ns/1ns
module t;
  function automatic logic [3:0] fx(input int a); logic [3:0] t; return t; endfunction
  logic [3:0] arr [0:($clog2(fx(2) + 4'd1) inside {32'b0000_0000_0000_0000_0000_0000_0000_000?})+2];
  initial #1 $display("asz=%0d", $size(arr));
  initial #5 $finish;
endmodule
