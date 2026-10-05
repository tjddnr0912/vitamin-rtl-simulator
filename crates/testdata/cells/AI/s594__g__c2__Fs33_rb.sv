`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic signed [32:0] AX [0:1] = '{-33'sd4, 33'sd2};
  logic [((AX[0] + 33'd0) == 33'h1_FFFF_FFFC) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #20 $finish;
endmodule
