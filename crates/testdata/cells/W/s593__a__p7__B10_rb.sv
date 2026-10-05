`timescale 1ns/1ns
module t;
  if (1) begin : gb
    localparam logic signed [63:0] GS = -64'sd4;
    logic [(GS ==? 4'sb1?00)+3:0] v;
    initial #1 $display("vb=%0d", $bits(v));
  end
  initial #5 $finish;
endmodule
