`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SP = 8'sd84;
  wire [7:0] r = {((SP ==? 4'sb?100)+1){4'b1010}};
  initial #1 $display("r=%b", r);
  initial #5 $finish;
endmodule
