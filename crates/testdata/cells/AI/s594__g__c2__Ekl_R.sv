`timescale 1ns/1ns
module m #(parameter int N = 2, parameter P = -(|{N{1'b1}}));
  initial #1 $display("P=%0d B=%0d", P, $bits(P));
endmodule
module t;
  m #(.P(-5)) u();
  initial #20 $finish;
endmodule
