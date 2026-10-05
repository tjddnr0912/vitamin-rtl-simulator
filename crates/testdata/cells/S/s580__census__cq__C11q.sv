`timescale 1ns/1ns
module t;
  localparam L2 = (4'b1100 ==? 4'b1?00);
  initial begin $display("C11q %b", L2); #1 $finish; end
  initial #100 $finish;
endmodule
