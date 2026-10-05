`timescale 1ns/1ns
module t;
  logic ad_S11 [((4'd15 + 4'd1) inside {5'b1?000}):0];
  initial begin #1 $display("S11A %0d", $size(ad_S11)); #1 $finish; end
  initial #100 $finish;
endmodule
