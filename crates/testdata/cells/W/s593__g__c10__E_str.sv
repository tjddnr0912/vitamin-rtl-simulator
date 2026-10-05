`timescale 1ns/1ns
module sub;
  localparam logic signed [7:0] X = -8'sd4;
  initial $display("h=%h d=%d o=%o b=%b", X, X, X, X);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
