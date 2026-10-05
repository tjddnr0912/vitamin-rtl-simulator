`timescale 1ns/1ns
module sub #(parameter logic signed [7:0] X = '0);
  initial $display("cmp=%0d mix=%0d", X > -5, X + 1'b1);
endmodule
module top;
  sub #(.X(-8'sd4)) u();
  initial #100 $finish;
endmodule
