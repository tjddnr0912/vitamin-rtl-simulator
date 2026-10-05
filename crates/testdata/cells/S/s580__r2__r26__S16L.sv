`timescale 1ns/1ns
module t;
  localparam LP_S16 = (4'b1100 inside {4'(4'b1?00)});
  initial begin #1 $display("S16L %b", LP_S16); #1 $finish; end
  initial #100 $finish;
endmodule
