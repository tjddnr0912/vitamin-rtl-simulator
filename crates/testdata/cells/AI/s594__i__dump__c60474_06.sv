`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  wire #((X + {N{1'b0}}) / 8'd50) w = 1'b1;
  always @(w) $display("w=%b t=%0t", w, $time);
  initial #50 $finish;
endmodule
