`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  int a [0:7] = '{0,1,2,3,4,5,6,7};
  initial $display("a=%0d", a[X+5]);
endmodule
module top;
  sub u();
  initial #100 $finish;
endmodule
