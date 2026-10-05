`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic signed [3:0] AS4 [0:1] = '{4'hF, 4'h1};
  localparam L = AS4[0] + 4'd0;
  initial #1 $display("L=%0d B=%0d", L, $bits(L));
  initial #20 $finish;
endmodule
