`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  typedef T u_t;
  u_t v;
  initial begin v = '1; #1 $display("D=%h", v); end
endmodule
module top;
  m u1(); m #(.T(logic [2:0])) u2();
  initial #5 $finish;
endmodule
