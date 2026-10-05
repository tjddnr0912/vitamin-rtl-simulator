`timescale 1ns/1ns
module t;
  logic [(2'b00 + 2'd3):0] v;
  initial #1 $display("rb=%0d", $bits(v));
  initial #40 $finish;
endmodule
