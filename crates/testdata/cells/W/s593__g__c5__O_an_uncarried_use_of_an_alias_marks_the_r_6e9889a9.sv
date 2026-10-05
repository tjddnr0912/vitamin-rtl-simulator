`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) ();
  parameter type U = T;
  typedef enum U { A = -1, B = 2 } e_t;
  e_t e; U u;
  initial begin e = A; u = -1; $display("e=%0d u=%0d", e, u); end
endmodule
module top; m #(.T(logic signed [7:0])) u (); initial #10 $finish; endmodule
