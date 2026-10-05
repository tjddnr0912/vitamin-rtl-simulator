`timescale 1ns/1ns
module t;
  logic [(4'b1100 inside {'b1?00}):0] wb_S03;
  initial begin #1 $display("S03B %0d", $bits(wb_S03)); #1 $finish; end
  initial #100 $finish;
endmodule
