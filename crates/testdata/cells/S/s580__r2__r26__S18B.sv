`timescale 1ns/1ns
module t;
  logic [(4'b1x00 > 4'd2):0] wb_S18;
  initial begin #1 $display("S18B %0d", $bits(wb_S18)); #1 $finish; end
  initial #100 $finish;
endmodule
