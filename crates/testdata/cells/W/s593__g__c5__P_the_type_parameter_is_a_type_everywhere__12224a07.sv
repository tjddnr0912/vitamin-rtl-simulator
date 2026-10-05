`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  function T inc(input T x); return x + 1; endfunction
  T v;
  initial begin v = inc(T'(254)); #1 $display("D=%h", v); end
endmodule
module top;
  m u1(); m #(.T(logic [3:0])) u2();
  initial #5 $finish;
endmodule
