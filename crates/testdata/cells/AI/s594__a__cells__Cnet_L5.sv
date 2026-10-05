`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  wire #((X + 2'b00) / 8'd50) w = 1'b1;
  always @(w) $display("w=%b t=%0t", w, $time);
  initial #40 $finish;
endmodule
