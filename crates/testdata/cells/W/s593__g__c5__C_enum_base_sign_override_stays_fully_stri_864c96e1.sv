`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) ();
  typedef enum T { EA = 8'hF0, EB = 8'h01 } e_t;
  e_t e;
  initial begin e = EA; $display("raw=%0d", e); end
  initial #10 $finish;
endmodule
module top; m #(.T(logic signed [7:0])) u(); endmodule
