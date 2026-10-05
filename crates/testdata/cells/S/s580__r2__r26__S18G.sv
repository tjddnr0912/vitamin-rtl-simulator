`timescale 1ns/1ns
module t;
  if (4'b1x00 > 4'd2) begin : gt_S18 initial #1 $display("S18G then"); end else begin : ge_S18 initial #1 $display("S18G else"); end
  initial begin #1  #1 $finish; end
  initial #100 $finish;
endmodule
