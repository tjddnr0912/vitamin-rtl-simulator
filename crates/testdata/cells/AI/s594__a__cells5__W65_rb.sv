`timescale 1ns/1ns
module t;
  localparam logic [64:0] A65 [0:1] = '{65'h1_0000_0000_0000_0002, 65'd2};
  logic [((A65[1] - 65'd1) == 65'd1) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #40 $finish;
endmodule
