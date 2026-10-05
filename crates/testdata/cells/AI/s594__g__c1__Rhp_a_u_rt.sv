`timescale 1ns/1ns
module t #(parameter int NH = 2);
  localparam logic [7:0] X = 8'hFC;
  localparam bit C = 1;
  initial #1 $display("RT=%0d", ((X + {NH{1'b0}}) == 8'hFC));
  initial #5 $finish;
endmodule
