`timescale 1ns/1ns
module t;
  localparam shortint TA [0:1] = '{-16'sd3, 16'sd2};
  logic [((TA[0] + 1'b0) == 16'hFFFD) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #40 $finish;
endmodule
