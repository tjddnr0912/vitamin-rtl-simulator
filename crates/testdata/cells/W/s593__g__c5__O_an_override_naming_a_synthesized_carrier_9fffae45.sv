`timescale 1ns/1ns
module m #(parameter W$x = 8) ();
  initial $display("w=%0d", W$x);
endmodule
module top; m #(.W$x(16)) u (); initial #10 $finish; endmodule
