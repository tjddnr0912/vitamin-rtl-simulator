`timescale 1ns/1ns
module t;
  if (4'b1100 inside {'x}) begin : gt_S13 initial #1 $display("S13G then"); end else begin : ge_S13 initial #1 $display("S13G else"); end
  initial begin #1  #1 $finish; end
  initial #100 $finish;
endmodule
