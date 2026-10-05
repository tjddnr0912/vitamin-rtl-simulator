`timescale 1ns/1ns
module t;
  typedef logic signed [5:0] s6_t;
  localparam s6_t TA [0:1] = '{-6'sd3, 6'sd2};
  logic [((TA[0] + 1'b0) == 6'h3D) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #40 $finish;
endmodule
