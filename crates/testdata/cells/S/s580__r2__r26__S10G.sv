`timescale 1ns/1ns
module t;
  if (4'b1100 !=? 4'b1?00) begin : gt_S10 initial #1 $display("S10G then"); end else begin : ge_S10 initial #1 $display("S10G else"); end
  initial begin #1  #1 $finish; end
  initial #100 $finish;
endmodule
