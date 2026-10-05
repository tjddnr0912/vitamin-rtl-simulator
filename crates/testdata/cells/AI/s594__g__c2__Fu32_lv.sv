`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [31:0] AW [0:1] = '{32'hFFFF_FFF0, 32'h2};
  localparam L = (AW[0] + AW[1]) >> 1;
  initial #1 $display("L=%0d B=%0d", L, $bits(L));
  initial #20 $finish;
endmodule
