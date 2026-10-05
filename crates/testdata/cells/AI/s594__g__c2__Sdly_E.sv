`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [7:0] A [0:1] = '{8'hFC, 8'h02};
  logic w = 0;
  assign #((X + (A[1] - 8'd2)) - 8'd250) w = 1'b1;
  initial begin #1 $display("t1 w=%b", w); #2 $display("t3 w=%b", w); end
  initial #20 $finish;
endmodule
