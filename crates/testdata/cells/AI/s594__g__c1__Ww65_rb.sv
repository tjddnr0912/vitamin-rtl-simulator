`timescale 1ns/1ns
module t;
  localparam logic signed [64:0] X = -4;
  localparam int N = 2;
  logic [((X + {N{1'b0}}) == 65'h1_FFFF_FFFF_FFFF_FFFC) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
