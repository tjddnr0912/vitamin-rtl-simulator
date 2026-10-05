`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  initial #1 $display("RT=%0d", ((X + 2'b00) == 8'hFC));
  initial #20 $finish;
endmodule
