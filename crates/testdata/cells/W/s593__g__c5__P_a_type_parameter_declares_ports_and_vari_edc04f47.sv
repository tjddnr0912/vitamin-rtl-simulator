`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) (input T d, output T q);
  assign q = d + 1;
endmodule
module top;
  logic [7:0] d, q;
  m u(.d(d), .q(q));
  initial begin d = 8'hfe; #1 $display("D=%h %0d", q, $bits(q)); #5 $finish; end
endmodule
