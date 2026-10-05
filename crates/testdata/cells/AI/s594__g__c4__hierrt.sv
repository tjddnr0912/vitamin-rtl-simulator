`timescale 1ns/1ns
module m; localparam logic [7:0] P = 8'h02; endmodule
module t;
  m u();
  localparam logic signed [7:0] X = -4;
  logic [((X | u.P) == 8'hFE) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
