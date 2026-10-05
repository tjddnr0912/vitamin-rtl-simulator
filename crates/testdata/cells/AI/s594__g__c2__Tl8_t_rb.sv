`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  logic [((C ? X : 8'h02) == 8'hFC) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #20 $finish;
endmodule
