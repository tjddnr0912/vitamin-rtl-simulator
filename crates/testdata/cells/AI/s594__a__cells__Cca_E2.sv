`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  wire w;
  assign #((X | A[1]) - 8'd252) w = 1'b1;
  always @(w) $display("w=%b t=%0t", w, $time);
  initial #40 $finish;
endmodule
