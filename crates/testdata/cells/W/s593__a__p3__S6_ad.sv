`timescale 1ns/1ns
module t;
  localparam logic [63:0] B = 64'h8000_0000_0000_000C;
  logic [3:0] arr [0:(B ==? 4'b1?00)+2];
  initial $display("asz=%0d", $size(arr));
  initial #5 $finish;
endmodule
