`timescale 1ns/1ns
module t;
  localparam LP_S10 = (4'b1100 !=? 4'b1?00);
  initial begin #1 $display("S10L %b", LP_S10); #1 $finish; end
  initial #100 $finish;
endmodule
