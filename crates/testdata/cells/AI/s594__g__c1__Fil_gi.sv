`timescale 1ns/1ns
module t;
  localparam int N = 2;
  if (('1 ^ {N{1'b0}}) == 2'b11) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #5 $finish;
endmodule
