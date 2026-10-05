`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [0:7] AD [0:1] = '{8'hFC, 8'h02};
  initial #1 $display("RT=%0d", ((X | AD[1]) == 8'hFE));
  initial #20 $finish;
endmodule
