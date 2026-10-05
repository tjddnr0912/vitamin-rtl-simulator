`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [15:0] W = 16'h00F0;
  initial #1 $display("RT=%0d", !(8'hFF + 8'd1 + 2'b00));
  initial #20 $finish;
endmodule
