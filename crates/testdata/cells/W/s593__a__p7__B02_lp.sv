`timescale 1ns/1ns
module t;
  typedef enum logic signed [3:0] {ES = -4'sd4, ET = 4'sd3} est4;
  localparam L = (ES ==? 8'sb1111_1?00);
  initial #1 $display("L=%b", L);
  initial #5 $finish;
endmodule
