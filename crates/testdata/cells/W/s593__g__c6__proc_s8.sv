`timescale 1ns/1ns
module sub #(parameter type T = logic [7:0]) ();
  localparam T X = -8'sd4;
  int k;
  initial begin k = 0; if (X < 0) k = 1; repeat (X + 6) k = k + 10; $display("k=%0d", k); end
endmodule
module top;
  sub #(.T(logic signed [7:0])) u();
  initial #100 $finish;
endmodule
