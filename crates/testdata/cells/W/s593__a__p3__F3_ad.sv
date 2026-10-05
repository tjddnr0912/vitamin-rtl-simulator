`timescale 1ns/1ns
module t;
  localparam logic [63:0] B2 = 64'h1_0000_000C;
  logic [3:0] arr [0:($unsigned(B2) == 4'd12)+2];
  initial $display("asz=%0d", $size(arr));
  initial #5 $finish;
endmodule
