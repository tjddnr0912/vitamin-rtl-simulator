`timescale 1ns/1ns
module t;
  localparam LP_S15 = (4'b1100 inside {(4'b1?00)});
  initial begin #1 $display("S15L %b", LP_S15); #1 $finish; end
  initial #100 $finish;
endmodule
