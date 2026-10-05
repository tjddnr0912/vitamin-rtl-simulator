`timescale 1ns/1ns
module t;
  function automatic logic [3:0] fx(input int a); logic [3:0] t; return t; endfunction
  if (fx(2) inside {4'b0?00}) begin : gi initial $display("GI=then"); end else begin : gi initial $display("GI=else"); end
  initial #5 $finish;
endmodule
