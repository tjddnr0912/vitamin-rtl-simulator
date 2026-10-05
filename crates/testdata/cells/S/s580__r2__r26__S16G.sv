`timescale 1ns/1ns
module t;
  if (4'b1100 inside {4'(4'b1?00)}) begin : gt_S16 initial #1 $display("S16G then"); end else begin : ge_S16 initial #1 $display("S16G else"); end
  initial begin #1  #1 $finish; end
  initial #100 $finish;
endmodule
