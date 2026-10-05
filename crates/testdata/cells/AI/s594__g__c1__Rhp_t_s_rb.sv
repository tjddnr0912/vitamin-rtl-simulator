`timescale 1ns/1ns
module t #(parameter int NH = 2);
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  logic [((C ? X : {NH{1'b0}}) == 8'hFC) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
