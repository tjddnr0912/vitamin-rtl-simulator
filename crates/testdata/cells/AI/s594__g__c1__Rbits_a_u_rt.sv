`timescale 1ns/1ns
module t;
  localparam logic [7:0] X = 8'hFC;
  localparam bit C = 1;
  initial #1 $display("RT=%0d", ((X + {$bits(2'b00){1'b0}}) == 8'hFC));
  initial #5 $finish;
endmodule
