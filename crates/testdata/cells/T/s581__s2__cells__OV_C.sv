`timescale 1ns/1ns
module m #(parameter P = 0) (); initial #1 $display("OV_P %b %0d", P, $bits(P)); endmodule
module t;
  m #(.P((4'b1100 == 4'b1100))) u();
endmodule
