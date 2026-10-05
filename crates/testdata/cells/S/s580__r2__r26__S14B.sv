`timescale 1ns/1ns
module t;
  logic [(4'bx100 ==? 4'b1?00):0] wb_S14;
  initial begin #1 $display("S14B %0d", $bits(wb_S14)); #1 $finish; end
  initial #100 $finish;
endmodule
