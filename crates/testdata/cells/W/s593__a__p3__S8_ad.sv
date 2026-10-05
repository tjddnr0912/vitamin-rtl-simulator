`timescale 1ns/1ns
module t;
  logic [3:0] arr [0:(8'sb1111_1100 ==? 4'sb1?00)+2];
  initial $display("asz=%0d", $size(arr));
  initial #5 $finish;
endmodule
