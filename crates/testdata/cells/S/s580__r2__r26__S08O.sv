`timescale 1ns/1ns
module m #(parameter P = 0) (); initial #1 $display("%m P=%b", P); endmodule
module t;
  m #(.P(4'bx100 inside {4'b1?00})) u_S08();
  initial begin #1  #1 $finish; end
  initial #100 $finish;
endmodule
