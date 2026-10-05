`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]) ();
  localparam int P = T'(8'hF0);
  localparam int Q = T'(8'hF0) - 1;
  initial $display("P=%0d Q=%0d", P, Q);
  initial #10 $finish;
endmodule
module top; m #(.T(logic signed [7:0])) u(); endmodule
