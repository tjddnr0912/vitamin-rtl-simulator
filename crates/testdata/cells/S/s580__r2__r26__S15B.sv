`timescale 1ns/1ns
module t;
  logic [(4'b1100 inside {(4'b1?00)}):0] wb_S15;
  initial begin #1 $display("S15B %0d", $bits(wb_S15)); #1 $finish; end
  initial #100 $finish;
endmodule
