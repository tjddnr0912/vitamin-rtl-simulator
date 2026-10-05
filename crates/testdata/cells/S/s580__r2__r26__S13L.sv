`timescale 1ns/1ns
module t;
  localparam LP_S13 = (4'b1100 inside {'x});
  initial begin #1 $display("S13L %b", LP_S13); #1 $finish; end
  initial #100 $finish;
endmodule
