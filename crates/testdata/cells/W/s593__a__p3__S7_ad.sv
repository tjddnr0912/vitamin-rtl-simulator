`timescale 1ns/1ns
module t;
  logic [3:0] arr [0:((4'd15 + 4'd1) ==? 5'b1?000)+2];
  initial $display("asz=%0d", $size(arr));
  initial #5 $finish;
endmodule
