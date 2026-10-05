`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int AI [0:1] = '{-4, 2};
  initial #1 $display("RT=%0d", ((AI[0] + 32'd0) == 32'hFFFF_FFFC));
  initial #20 $finish;
endmodule
