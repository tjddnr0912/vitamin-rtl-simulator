`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] SN = -8'sd60;
  if (SN ==? 4'b?100) begin : gi initial $display("GI=then"); end else begin : gi initial $display("GI=else"); end
  initial #5 $finish;
endmodule
