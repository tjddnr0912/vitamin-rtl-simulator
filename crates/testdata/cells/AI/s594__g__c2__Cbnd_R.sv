`timescale 1ns/1ns
module t;
  localparam int N = 2;
  logic [({N{1'b1}} + 2'd2):0] b;
  initial #1 $display("bb=%0d", $bits(b));
  initial #20 $finish;
endmodule
