`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SP = 8'sd84;
  localparam L = (SP ==? 4'sb?100);
  initial $display("L=%b", L);
  initial #5 $finish;
endmodule
