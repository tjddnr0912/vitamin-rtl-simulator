`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam T X = -4;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  localparam int N = 2;
  localparam bit C = 1;
  logic [((X | A[1]) ==? 8'b1111_11?0)+3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
endmodule
module t;
  m u();
  initial #5 $finish;
endmodule
