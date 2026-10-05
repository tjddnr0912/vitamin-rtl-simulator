`timescale 1ns/1ns
module t;
  if (4'bx100 ==? 4'b1?00) begin : gt_S14 initial #1 $display("S14G then"); end else begin : ge_S14 initial #1 $display("S14G else"); end
  initial begin #1  #1 $finish; end
  initial #100 $finish;
endmodule
