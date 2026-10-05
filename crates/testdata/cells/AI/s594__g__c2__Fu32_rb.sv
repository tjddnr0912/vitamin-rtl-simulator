`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [31:0] AW [0:1] = '{32'hFFFF_FFF0, 32'h2};
  logic [(((AW[0] + AW[1]) >> 1) == 32'h7FFF_FFF9) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #20 $finish;
endmodule
