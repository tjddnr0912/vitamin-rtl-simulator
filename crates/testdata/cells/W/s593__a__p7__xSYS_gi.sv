`timescale 1ns/1ns
module t;
  function automatic logic [3:0] fx(input int a); logic [3:0] t; return t; endfunction
  if ($clog2(fx(2) + 4'd1) inside {32'b0000_0000_0000_0000_0000_0000_0000_000?}) begin : gi initial #1 $display("GI=then"); end else begin : gi initial #1 $display("GI=else"); end
  initial #5 $finish;
endmodule
