`timescale 1ns/1ns
module t;
  localparam LP_S17 = (4'b1100 ==? 4'(4'b1?00));
  initial begin #1 $display("S17L %b", LP_S17); #1 $finish; end
  initial #100 $finish;
endmodule
