`timescale 1ns/1ns
module t;
  if ((4'd15 + 4'd1) inside {5'b1?000}) begin : gt_S11 initial #1 $display("S11G then"); end else begin : ge_S11 initial #1 $display("S11G else"); end
  initial begin #1  #1 $finish; end
  initial #100 $finish;
endmodule
