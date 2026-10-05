`timescale 1ns/1ns
module t;
  logic [(4'b1100 inside {4'b1?00}):0] wb_S01;
  initial begin #1 $display("S01B %0d", $bits(wb_S01)); #1 $finish; end
  initial #100 $finish;
endmodule
