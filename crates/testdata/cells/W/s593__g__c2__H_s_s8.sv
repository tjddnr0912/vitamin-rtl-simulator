`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0], parameter T X = '0);
  initial $display("sg=%0d us=%0d", $signed(X), $unsigned(X));
endmodule
module top;
  sub #(.T(logic signed [7:0]), .X(-8'sd4)) u();
  initial #100 $finish;
endmodule
