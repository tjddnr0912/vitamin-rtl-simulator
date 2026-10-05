`timescale 1ns/1ns
module t;
  if (4'b1100 == 4'b1?00) begin : gt_S09 initial #1 $display("S09G then"); end else begin : ge_S09 initial #1 $display("S09G else"); end
  initial begin #1  #1 $finish; end
  initial #100 $finish;
endmodule
