`timescale 1ns/1ns
module t;
  localparam time TA [0:1] = '{64'd5, 64'd7};
  logic [((TA[0] + 1'b0) == 64'd5) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #40 $finish;
endmodule
