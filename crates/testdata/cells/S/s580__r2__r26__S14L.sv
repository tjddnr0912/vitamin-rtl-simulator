`timescale 1ns/1ns
module t;
  localparam LP_S14 = (4'bx100 ==? 4'b1?00);
  initial begin #1 $display("S14L %b", LP_S14); #1 $finish; end
  initial #100 $finish;
endmodule
