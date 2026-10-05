`timescale 1ns/1ns
module t;
  localparam int N = 2;
  logic [({N{1'b0}} + 2'd3):0] v;
  initial #1 $display("rb=%0d", $bits(v));
  initial #40 $finish;
endmodule
