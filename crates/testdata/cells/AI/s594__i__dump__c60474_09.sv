`timescale 1ns/1ns
module m #(parameter int N = 40, parameter P = (|{N{1'b1}}) + {N{1'b0}});
  initial #1 $display("P=%0d B=%0d", P, $bits(P));
endmodule
module t;
  m u();
  initial #50 $finish;
endmodule
