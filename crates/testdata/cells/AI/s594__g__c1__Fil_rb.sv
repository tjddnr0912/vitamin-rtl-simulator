`timescale 1ns/1ns
module t;
  localparam int N = 2;
  logic [(('1 ^ {N{1'b0}}) == 2'b11) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
