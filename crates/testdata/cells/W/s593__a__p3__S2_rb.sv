`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SP = 8'sd84;
  logic [(SP ==? 4'sb?100)+3:0] v;
  initial $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
