`timescale 1ns/1ns
module t;
  localparam LP_S04 = (4'b1100 inside {4'b0000, 4'b1?00});
  initial begin #1 $display("S04L %b", LP_S04); #1 $finish; end
  initial #100 $finish;
endmodule
