`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  logic [((X + {N{16'h0}}) == 32'hFC) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
