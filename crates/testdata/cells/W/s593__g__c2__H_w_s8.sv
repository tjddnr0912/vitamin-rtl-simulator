`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0], parameter T X = '0);
  logic [X+8:0] v;
  initial $display("vb=%0d", $bits(v));
endmodule
module top;
  sub #(.T(logic signed [7:0]), .X(-8'sd4)) u();
  initial #100 $finish;
endmodule
