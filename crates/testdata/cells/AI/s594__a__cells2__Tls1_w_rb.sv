`timescale 1ns/1ns
module t;
  localparam logic signed TA [0:1] = '{1'b1, 1'b0};
  logic [((TA[0] + 1'b1) == 1'b0) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #40 $finish;
endmodule
