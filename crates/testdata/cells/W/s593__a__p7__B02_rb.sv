`timescale 1ns/1ns
module t;
  typedef enum logic signed [3:0] {ES = -4'sd4, ET = 4'sd3} est4;
  logic [(ES ==? 8'sb1111_1?00)+3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
