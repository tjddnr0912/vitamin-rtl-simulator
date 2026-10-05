`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  wire w;
  assign #((X + {N{1'b0}}) * 1ns) w = 1'b1;
  always @(w) $display("w=%b t=%0t", w, $time);
  initial #400 $finish;
endmodule
