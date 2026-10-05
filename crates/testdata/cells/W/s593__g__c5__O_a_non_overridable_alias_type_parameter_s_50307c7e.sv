`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) ();
  localparam type U = T;
  U u; initial begin u = -1; $display("u=%0d neg=%0d", u, (u<0)); end
endmodule
module top; m #(.T(logic signed [7:0])) u (); initial #10 $finish; endmodule
