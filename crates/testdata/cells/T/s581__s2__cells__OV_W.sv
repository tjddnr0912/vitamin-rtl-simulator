`timescale 1ns/1ns
module m #(parameter P = 0) (); initial #1 $display("OV_P %b %0d", P, $bits(P)); endmodule
module t;
  m #(.P(((4'd15 + 4'd1) ==? 5'b1?000))) u();
endmodule
