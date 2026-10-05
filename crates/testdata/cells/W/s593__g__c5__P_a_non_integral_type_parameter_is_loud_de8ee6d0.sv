`timescale 1ns/1ns
typedef struct packed { logic [3:0] a; logic [3:0] b; } st_t;
module m #(parameter type T = st_t);
  T v;
endmodule
module top; m u(); endmodule
