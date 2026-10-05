`timescale 1ns/1ns
module t;
  localparam LP_S18 = (4'b1x00 > 4'd2);
  initial begin #1 $display("S18L %b", LP_S18); #1 $finish; end
  initial #100 $finish;
endmodule
