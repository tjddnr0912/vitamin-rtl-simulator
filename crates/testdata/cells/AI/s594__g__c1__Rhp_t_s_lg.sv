`timescale 1ns/1ns
module t #(parameter int NH = 2);
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  localparam L = ((C ? X : {NH{1'b0}}) > 8'd100);
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
