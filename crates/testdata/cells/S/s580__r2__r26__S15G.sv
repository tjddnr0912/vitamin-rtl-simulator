`timescale 1ns/1ns
module t;
  if (4'b1100 inside {(4'b1?00)}) begin : gt_S15 initial #1 $display("S15G then"); end else begin : ge_S15 initial #1 $display("S15G else"); end
  initial begin #1  #1 $finish; end
  initial #100 $finish;
endmodule
