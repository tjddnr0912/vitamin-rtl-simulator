`timescale 1ns/1ns
module m #(parameter type T = logic [7:0], parameter int N = 2);
  T v [N];
  initial begin v[0] = 8'h11; v[N-1] = 8'h22; #1 $display("D=%h %h %0d", v[0], v[N-1], $bits(T)); end
endmodule
module top;
  m #(logic [15:0], 3) u();
  initial #5 $finish;
endmodule
