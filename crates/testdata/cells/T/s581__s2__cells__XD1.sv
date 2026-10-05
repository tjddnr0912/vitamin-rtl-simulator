`timescale 1ns/1ns
module t;
  logic a [4'bx : 0];
  initial #1 $display("XD1 %0d", $size(a));
endmodule
