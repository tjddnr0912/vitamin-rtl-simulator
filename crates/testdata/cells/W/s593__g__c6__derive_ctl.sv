`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  localparam Y = X + 1;
  localparam Z = X * 2 + 1;
  initial $display("Y=%0d Z=%0d", Y, Z);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
