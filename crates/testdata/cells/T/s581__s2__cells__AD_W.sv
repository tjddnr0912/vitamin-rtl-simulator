`timescale 1ns/1ns
module t;
  logic a [((4'd15 + 4'd1) ==? 5'b1?000) : 0];
  initial #1 $display("AD_W %0d", $size(a));
endmodule
