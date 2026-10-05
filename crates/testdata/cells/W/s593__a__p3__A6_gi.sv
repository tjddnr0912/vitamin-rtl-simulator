`timescale 1ns/1ns
module t;
  function automatic logic [3:0] fr(input int a); if (a > 5) fr = 4'd1; endfunction
  if (fr(2) inside {4'b0?00}) begin : gi initial $display("GI=then"); end else begin : gi initial $display("GI=else"); end
  initial #5 $finish;
endmodule
