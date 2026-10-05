`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam logic [31:0] AW [0:1] = '{32'hFFFF_FFF0, 32'h2};
  if (((AW[0] + AW[1]) >> 1) == 32'h7FFF_FFF9) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #20 $finish;
endmodule
