`timescale 1ns/1ns
module m #(parameter int N = 1);
  localparam logic signed [7:0] X = -4;
  localparam V = X + {N{3'b000}};
  initial #1 $display("V=%0d B=%0d", V, $bits(V));
endmodule
module t;
  m #(.N(3)) u();
  initial #20 $finish;
endmodule
