`timescale 1ns/1ns
module t;
  typedef enum logic signed [3:0] {ES = -4'sd4, ET = 4'sd3} est4;
  logic [3:0] arr [0:(ES ==? 8'sb1111_1?00)+2];
  initial #1 $display("asz=%0d", $size(arr));
  initial #5 $finish;
endmodule
