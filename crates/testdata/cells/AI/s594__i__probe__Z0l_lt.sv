`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 0;
  localparam bit C = 1;
  localparam L = ((C ? X : {1'b0, {0{1'b0}}}) == 8'hFC);
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
