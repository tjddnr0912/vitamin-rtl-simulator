`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit AB [0:1] = '{1'b1, 1'b0};
  logic [((X + AB[0]) == 8'hFD) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #20 $finish;
endmodule
