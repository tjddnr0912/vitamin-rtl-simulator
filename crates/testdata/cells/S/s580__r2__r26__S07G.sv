`timescale 1ns/1ns
module t;
  if (4'b1100 ==? {2'b1?, 2'b00}) begin : gt_S07 initial #1 $display("S07G then"); end else begin : ge_S07 initial #1 $display("S07G else"); end
  initial begin #1  #1 $finish; end
  initial #100 $finish;
endmodule
