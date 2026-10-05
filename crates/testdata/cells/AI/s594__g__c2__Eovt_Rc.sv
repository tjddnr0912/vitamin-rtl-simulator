`timescale 1ns/1ns
module m #(parameter logic [15:0] P = 0);
  initial #1 $display("P=%0d", P);
endmodule
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  m #(.P((X + {N{1'b0}}) == 8'hFC)) u();
  initial #20 $finish;
endmodule
