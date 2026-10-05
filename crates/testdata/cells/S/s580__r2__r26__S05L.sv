`timescale 1ns/1ns
module t;
  localparam LP_S05 = (4'b1100 inside {{2'b1?, 2'b00}});
  initial begin #1 $display("S05L %b", LP_S05); #1 $finish; end
  initial #100 $finish;
endmodule
