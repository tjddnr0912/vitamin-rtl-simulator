`timescale 1ns/1ns
module t;
  localparam LP_S12 = (4'b0100 inside {4'b1?00});
  initial begin #1 $display("S12L %b", LP_S12); #1 $finish; end
  initial #100 $finish;
endmodule
