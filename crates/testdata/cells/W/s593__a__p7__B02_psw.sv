`timescale 1ns/1ns
module t;
  typedef enum logic signed [3:0] {ES = -4'sd4, ET = 4'sd3} est4;
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: (ES ==? 8'sb1111_1?00)+3];
  initial #1 $display("ps=%b", ps);
  initial #5 $finish;
endmodule
