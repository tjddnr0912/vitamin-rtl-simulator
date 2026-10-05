`timescale 1ns/1ns
module t;
  if (1) begin : gb
    localparam logic signed [63:0] GS = -64'sd4;
    wire [7:0] s = 8'hA5;
    wire [7:0] ps = s[0 +: (GS ==? 4'sb1?00)+3];
    initial #1 $display("ps=%b", ps);
  end
  initial #5 $finish;
endmodule
