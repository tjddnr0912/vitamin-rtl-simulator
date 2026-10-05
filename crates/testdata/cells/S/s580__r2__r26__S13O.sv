`timescale 1ns/1ns
module m #(parameter P = 0) (); initial #1 $display("%m P=%b", P); endmodule
module t;
  m #(.P(4'b1100 inside {'x})) u_S13();
  initial begin #1  #1 $finish; end
  initial #100 $finish;
endmodule
