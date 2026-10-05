`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [3:0] A4 [0:1] = '{8'hFF, 4'h1};
  logic [((A4[0] + 4'd1) == 4'd0) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #20 $finish;
endmodule
