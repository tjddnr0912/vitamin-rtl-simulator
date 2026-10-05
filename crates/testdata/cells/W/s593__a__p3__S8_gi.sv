`timescale 1ns/1ns
module t;
  if (8'sb1111_1100 ==? 4'sb1?00) begin : gi initial $display("GI=then"); end else begin : gi initial $display("GI=else"); end
  initial #5 $finish;
endmodule
