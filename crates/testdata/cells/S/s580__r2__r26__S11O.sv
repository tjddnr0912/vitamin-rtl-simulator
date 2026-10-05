`timescale 1ns/1ns
module m #(parameter P = 0) (); initial #1 $display("%m P=%b", P); endmodule
module t;
  m #(.P((4'd15 + 4'd1) inside {5'b1?000})) u_S11();
  initial begin #1  #1 $finish; end
  initial #100 $finish;
endmodule
