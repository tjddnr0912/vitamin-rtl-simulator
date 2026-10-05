`timescale 1ns/1ns
module t;
  if (4'bx100 inside {4'b1?00}) begin : gt_S08 initial #1 $display("S08G then"); end else begin : ge_S08 initial #1 $display("S08G else"); end
  initial begin #1  #1 $finish; end
  initial #100 $finish;
endmodule
