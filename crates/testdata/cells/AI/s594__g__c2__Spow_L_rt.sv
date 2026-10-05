`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [15:0] W = 16'h00F0;
  initial #1 $display("RT=%0d", 2 ** ((X + 2'b00) - 8'd250));
  initial #20 $finish;
endmodule
