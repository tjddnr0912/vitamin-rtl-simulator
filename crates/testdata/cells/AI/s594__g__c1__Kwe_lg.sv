`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  localparam L = ((8'hFF + A[1]) > 8'd100);
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
