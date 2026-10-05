`timescale 1ns/1ns
module t;
  localparam int X = -4;
  localparam int N = 2;
  logic [((X + {N{1'b0}}) == 32'hFFFF_FFFC) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
