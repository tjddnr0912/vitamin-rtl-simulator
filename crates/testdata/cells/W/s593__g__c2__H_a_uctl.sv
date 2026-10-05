`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0], parameter T X = -8'sd4);
  initial $display("shr=%0d mul=%0d div=%0d mod=%0d", X >>> 1, X * 3, X / 2, X % 3);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
