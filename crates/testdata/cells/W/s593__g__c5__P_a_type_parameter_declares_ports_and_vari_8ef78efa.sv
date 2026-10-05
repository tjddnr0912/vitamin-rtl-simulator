`timescale 1ns/1ns
module m #(type T = logic [7:0]);
  T v;
  initial begin v = 8'hab; #1 $display("D=%h", v); end
endmodule
module top;
  m u(); m #(.T(logic [11:0])) u2();
  initial #5 $finish;
endmodule
