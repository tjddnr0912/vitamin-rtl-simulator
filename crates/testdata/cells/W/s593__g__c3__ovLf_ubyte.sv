`timescale 1ns/1ns
typedef logic signed [7:0] s8_t;
package p; typedef logic signed [7:0] ps8_t; endpackage
typedef struct packed signed { logic [3:0] a; logic [3:0] b; } ss_t;
typedef struct packed { logic [3:0] a; logic [3:0] b; } su_t;
module sub #(parameter type T = logic [3:0]) ();
  localparam T X = '1;
  initial $display("X=%0d lt0=%0d b=%0d shr=%0d", X, X < 0, $bits(X), X >>> 1);
endmodule
module top;
  sub #(.T(byte unsigned)) u();
  initial #100 $finish;
endmodule
