`timescale 1ns/1ns
module m #(parameter W = 8);
  typedef logic [W-1:0] t;
  t v;
  initial begin v = '1; #1 $display("D=%h %0d", v, $bits(t)); end
endmodule
module top;
  m u1(); m #(.W(3)) u2();
  initial #5 $finish;
endmodule
