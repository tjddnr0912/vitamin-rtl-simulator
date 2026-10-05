`timescale 1ns/1ns
module t;
  localparam logic [7:0] X = 8'hFC;
  localparam bit C = 1;
  localparam string S = "ab";
  initial #1 $display("RT=%0d", (((X | 8'd0) + S.len()) == 32'd254));
  initial #5 $finish;
endmodule
