`timescale 1ns/1ns
module t;
  if (1) begin : gb
    localparam logic signed [63:0] GS = -64'sd4;
    if (GS ==? 4'sb1?00) begin : gi initial #1 $display("GI=then"); end else begin : gi initial #1 $display("GI=else"); end
  end
  initial #5 $finish;
endmodule
