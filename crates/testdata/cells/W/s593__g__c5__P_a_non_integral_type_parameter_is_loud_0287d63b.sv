`timescale 1ns/1ns
typedef struct packed { logic [3:0] a; logic [3:0] b; } st_t;
module m #(parameter type T = logic [7:0]);
  T v;
endmodule
module top; m #(.T(st_t)) u(); endmodule
