`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) ();
  logic [7:0] q;
  initial begin q = T'(q); $display("u=%b", q); end
  initial #10 $finish;
endmodule
module top; m #(.T(bit [7:0])) u(); endmodule
