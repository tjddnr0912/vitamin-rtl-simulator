`timescale 1ns/1ns
module m #(parameter logic [15:0] P = 0);
  initial #1 $display("P=%0d", P);
endmodule
module t;
  localparam logic signed [7:0] X = -4;
  m #(.P(X + 2'b00)) u();
  initial #20 $finish;
endmodule
