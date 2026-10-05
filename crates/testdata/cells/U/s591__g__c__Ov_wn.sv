module c #(parameter [64:0] P = 65'h1_0000_0000_0000_0009) ();
  initial #1 $display("%m P=%0d b=%0d", P, $bits(P));
  if (P > 65'd100) begin : t initial #1 $display("%m big"); end
endmodule
module top;
  c #(.P(3)) u ();
  initial #100 $finish;
endmodule
