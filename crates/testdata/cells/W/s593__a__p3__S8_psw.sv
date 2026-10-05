`timescale 1ns/1ns
module t;
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: (8'sb1111_1100 ==? 4'sb1?00)+3];
  initial #1 $display("ps=%b", ps);
  initial #5 $finish;
endmodule
