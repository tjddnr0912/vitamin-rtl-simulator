`timescale 1ns/1ns
module t;
  localparam LP_S01 = (4'b1100 inside {4'b1?00});
  initial begin #1 $display("S01L %b", LP_S01); #1 $finish; end
  initial #100 $finish;
endmodule
