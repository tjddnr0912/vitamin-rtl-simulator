`timescale 1ns/1ns
module t;
  if (4'b1100 inside {{2'b1?, 2'b00}}) begin : gt_S05 initial #1 $display("S05G then"); end else begin : ge_S05 initial #1 $display("S05G else"); end
  initial begin #1  #1 $finish; end
  initial #100 $finish;
endmodule
