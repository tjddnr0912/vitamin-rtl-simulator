`timescale 1ns/1ns
module t;
  logic [(4'b0100 inside {4'b1?00}):0] wb_S12;
  initial begin #1 $display("S12B %0d", $bits(wb_S12)); #1 $finish; end
  initial #100 $finish;
endmodule
