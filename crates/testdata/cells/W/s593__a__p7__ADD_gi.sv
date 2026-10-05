`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SA = -8'sd4;
  if ((SA + 8'sd0) ==? 4'sb1?00) begin : gi initial #1 $display("GI=then"); end else begin : gi initial #1 $display("GI=else"); end
  initial #5 $finish;
endmodule
