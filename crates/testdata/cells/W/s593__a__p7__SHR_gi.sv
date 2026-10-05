`timescale 1ns/1ns
module t;
  localparam logic signed [3:0] SA = -4'sd2;
  if ((SA >>> 1) ==? 4'sb111?) begin : gi initial #1 $display("GI=then"); end else begin : gi initial #1 $display("GI=else"); end
  initial #5 $finish;
endmodule
