`timescale 1ns/1ns
module m #(parameter W = 8);
  typedef logic [W-1:0] t;
  localparam int B = $bits(t);
  logic [B*2-1:0] w;
  initial begin w = t'(-1); #1 $display("D=%h B=%0d", w, B); end
endmodule
module top;
  m u1(); m #(.W(3)) u2();
  initial #5 $finish;
endmodule
