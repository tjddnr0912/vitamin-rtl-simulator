`timescale 1ns/1ns
module t;
  if (4'b1100 inside {4'sb1?00}) begin : gt_S02 initial #1 $display("S02G then"); end else begin : ge_S02 initial #1 $display("S02G else"); end
  initial begin #1  #1 $finish; end
  initial #100 $finish;
endmodule
