`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  for (genvar g = 2; g < 3; g = g + 1) begin : gl
  if ((X + {g{1'b0}}) == 8'hFC) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  end
  initial #5 $finish;
endmodule
