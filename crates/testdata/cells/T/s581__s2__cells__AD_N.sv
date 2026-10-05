`timescale 1ns/1ns
module t;
  logic a [((4'd15 + 4'd1) inside {5'b0?000}) : 0];
  initial #1 $display("AD_N %0d", $size(a));
endmodule
