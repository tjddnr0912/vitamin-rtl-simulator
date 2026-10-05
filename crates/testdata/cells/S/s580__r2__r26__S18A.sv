`timescale 1ns/1ns
module t;
  logic ad_S18 [(4'b1x00 > 4'd2):0];
  initial begin #1 $display("S18A %0d", $size(ad_S18)); #1 $finish; end
  initial #100 $finish;
endmodule
