`timescale 1ns/1ns
module m #(parameter logic P = 1'b0) (); initial $display("C29q %b", P); endmodule
module t;
  m #(.P(4'b1100 ==? 4'b1?00)) u();
  initial #1 $finish;
  initial #100 $finish;
endmodule
