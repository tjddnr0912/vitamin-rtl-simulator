`timescale 1ns/1ns
module t;
  localparam logic [7:0] X = 8'hFC;
  localparam bit C = 1;
  localparam string S = "ab";
  localparam L = (((X | 8'd0) + S.len()) == 32'd254);
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
