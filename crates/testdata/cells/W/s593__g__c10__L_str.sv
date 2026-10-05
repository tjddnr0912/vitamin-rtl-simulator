`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  initial $display("h=%h d=%d o=%o b=%b", X, X, X, X);
endmodule
module top;
  sub #(.T(logic signed [7:0])) u();
  initial #100 $finish;
endmodule
