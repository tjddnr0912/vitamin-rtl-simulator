`timescale 1ns/1ns
module t;
  if (4'b1100 inside {'b1?00}) begin : gt_S03 initial #1 $display("S03G then"); end else begin : ge_S03 initial #1 $display("S03G else"); end
  initial begin #1  #1 $finish; end
  initial #100 $finish;
endmodule
