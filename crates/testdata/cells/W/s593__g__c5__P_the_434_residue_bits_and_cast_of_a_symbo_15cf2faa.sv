`timescale 1ns/1ns
module m #(parameter W = 8) (input logic [W-1:0] d, output logic [W-1:0] q);
  typedef logic [W-1:0] t;
  t v;
  assign q = d + 1;
  initial begin v = t'(300); #1 $display("D=%h %h", v, q); end
endmodule
module top;
  logic [3:0] d, q;
  m #(.W(4)) u(.d(d), .q(q));
  initial begin d = 4'he; #5 $finish; end
endmodule
