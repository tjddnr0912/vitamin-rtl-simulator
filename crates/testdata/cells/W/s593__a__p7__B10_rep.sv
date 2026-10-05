`timescale 1ns/1ns
module t;
  if (1) begin : gb
    localparam logic signed [63:0] GS = -64'sd4;
    wire [7:0] r = {((GS ==? 4'sb1?00)+1){4'b1010}};
    initial #1 $display("r=%b", r);
  end
  initial #5 $finish;
endmodule
