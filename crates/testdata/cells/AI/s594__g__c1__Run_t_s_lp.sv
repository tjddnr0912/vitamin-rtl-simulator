`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  localparam NU = 2;
  localparam L = ((C ? X : {NU{1'b0}}) == 8'hFC);
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
