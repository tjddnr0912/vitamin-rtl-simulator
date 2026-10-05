`timescale 1ns/1ns
module t;
  localparam integer TA [0:1] = '{-5, 7};
  logic [((TA[0] + 1'b0) == 32'hFFFFFFFB) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #40 $finish;
endmodule
