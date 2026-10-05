`timescale 1ns/1ns
module t;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  initial #1 $display("rep=%h", {((A[1] - 8'd2) + 2'd3){4'hF}});
  initial #40 $finish;
endmodule
