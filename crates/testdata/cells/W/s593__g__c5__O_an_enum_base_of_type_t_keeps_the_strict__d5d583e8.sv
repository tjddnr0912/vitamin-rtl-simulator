`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) ();
  typedef enum T { A = 1, B = 2 } e_t;
  e_t e;
  initial begin e = B; $display("enum=%0d", e); end
endmodule
module top; m #(.T(logic signed [7:0])) u (); initial #10 $finish; endmodule
