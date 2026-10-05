`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SP = 8'sd84;
  if (SP ==? 4'sb?100) begin : gi initial $display("GI=then"); end else begin : gi initial $display("GI=else"); end
  initial #5 $finish;
endmodule
