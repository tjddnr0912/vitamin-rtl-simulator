`timescale 1ns/1ns
module t;
  logic [(4'b1100 inside {'x}):0] wb_S13;
  initial begin #1 $display("S13B %0d", $bits(wb_S13)); #1 $finish; end
  initial #100 $finish;
endmodule
