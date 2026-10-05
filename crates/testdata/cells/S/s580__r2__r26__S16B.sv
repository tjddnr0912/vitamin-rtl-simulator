`timescale 1ns/1ns
module t;
  logic [(4'b1100 inside {4'(4'b1?00)}):0] wb_S16;
  initial begin #1 $display("S16B %0d", $bits(wb_S16)); #1 $finish; end
  initial #100 $finish;
endmodule
