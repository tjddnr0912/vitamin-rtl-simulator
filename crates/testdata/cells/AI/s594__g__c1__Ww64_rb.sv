`timescale 1ns/1ns
module t;
  localparam longint X = -4;
  localparam int N = 2;
  logic [((X + {N{1'b0}}) == 64'hFFFF_FFFF_FFFF_FFFC) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
