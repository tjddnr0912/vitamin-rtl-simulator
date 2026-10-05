`timescale 1ns/1ns
typedef enum logic [1:0] {A=0, B=1, C=2} e_t;
module m #(parameter type T = e_t);
  T v;
endmodule
module top; m u(); endmodule
