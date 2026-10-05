`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  initial begin #((X | A[1]) * 1ns); $display("fired t=%0t", $time); end
  initial #400 $finish;
endmodule
