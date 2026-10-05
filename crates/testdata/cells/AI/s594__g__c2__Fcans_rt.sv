`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic signed [3:0] AS4 [0:1] = '{4'hF, 4'h1};
  initial #1 $display("RT=%0d", ((AS4[0] + 4'd0) == 4'hF));
  initial #20 $finish;
endmodule
