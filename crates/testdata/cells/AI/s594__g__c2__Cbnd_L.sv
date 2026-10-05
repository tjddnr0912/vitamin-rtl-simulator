`timescale 1ns/1ns
module t;
  logic [(2'b11 + 2'd2):0] b;
  initial #1 $display("bb=%0d", $bits(b));
  initial #20 $finish;
endmodule
