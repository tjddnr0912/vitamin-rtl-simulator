`timescale 1ns/1ns
module t;
  logic [(4'b1100 ==? 4'(4'b1?00)):0] wb_S17;
  initial begin #1 $display("S17B %0d", $bits(wb_S17)); #1 $finish; end
  initial #100 $finish;
endmodule
