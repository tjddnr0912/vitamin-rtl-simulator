`timescale 1ns/1ns
module t;
  logic [(4'b1100 inside {4'sb1?00}):0] wb_S02;
  initial begin #1 $display("S02B %0d", $bits(wb_S02)); #1 $finish; end
  initial #100 $finish;
endmodule
