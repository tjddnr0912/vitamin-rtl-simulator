`timescale 1ns/1ns
module m #(parameter type T = real);
  T v;
endmodule
module top; m u(); endmodule
