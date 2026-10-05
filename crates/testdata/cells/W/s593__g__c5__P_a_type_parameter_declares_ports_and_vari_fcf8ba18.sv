`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) (input T d, output T q);
  assign q = d + 1;
  initial #2 $display("D=%0d", $bits(T));
endmodule
module top;
  logic [15:0] d, q;
  m #(.T(logic [15:0])) u(.d(d), .q(q));
  initial begin d = 16'hfffe; #1 $display("D=%h", q); #5 $finish; end
endmodule
