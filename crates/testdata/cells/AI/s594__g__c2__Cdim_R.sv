`timescale 1ns/1ns
module t;
  localparam int N = 2;
  logic d [({N{1'b1}} + 2'd2):0];
  initial #1 $display("ds=%0d", $size(d));
  initial #20 $finish;
endmodule
