`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  if (1) begin : gb
  localparam L = X + {N{1'b0}};
  initial #1 $display("L=%0d B=%0d", L, $bits(L));
  end
  initial #5 $finish;
endmodule
