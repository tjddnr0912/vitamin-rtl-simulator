`timescale 1ns/1ns
module t;
  localparam byte TA [0:1] = '{-8'sd3, 8'sd2};
  logic [((TA[0] + 1'b0) == 8'hFD) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #40 $finish;
endmodule
