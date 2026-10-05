`timescale 1ns/1ns
module top;
  localparam type T = logic [5:0];
  T v;
  initial begin v = '1; #1 $display("D=%h %0d", v, $bits(T)); #5 $finish; end
endmodule
