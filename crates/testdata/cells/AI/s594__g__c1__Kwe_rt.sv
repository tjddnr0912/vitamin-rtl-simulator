`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  initial #1 $display("RT=%0d", ((8'hFF + A[1]) == 8'h01));
  initial #5 $finish;
endmodule
