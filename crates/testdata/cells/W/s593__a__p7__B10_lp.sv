`timescale 1ns/1ns
module t;
  if (1) begin : gb
    localparam logic signed [63:0] GS = -64'sd4;
    localparam L = (GS ==? 4'sb1?00);
    initial #1 $display("L=%b", L);
  end
  initial #5 $finish;
endmodule
