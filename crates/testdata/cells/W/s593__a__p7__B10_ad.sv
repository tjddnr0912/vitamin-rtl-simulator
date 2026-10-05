`timescale 1ns/1ns
module t;
  if (1) begin : gb
    localparam logic signed [63:0] GS = -64'sd4;
    logic [3:0] arr [0:(GS ==? 4'sb1?00)+2];
    initial #1 $display("asz=%0d", $size(arr));
  end
  initial #5 $finish;
endmodule
