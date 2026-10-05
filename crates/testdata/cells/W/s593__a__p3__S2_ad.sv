`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SP = 8'sd84;
  logic [3:0] arr [0:(SP ==? 4'sb?100)+2];
  initial $display("asz=%0d", $size(arr));
  initial #5 $finish;
endmodule
