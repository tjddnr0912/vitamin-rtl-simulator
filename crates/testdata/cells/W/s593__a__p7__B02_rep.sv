`timescale 1ns/1ns
module t;
  typedef enum logic signed [3:0] {ES = -4'sd4, ET = 4'sd3} est4;
  wire [7:0] r = {((ES ==? 8'sb1111_1?00)+1){4'b1010}};
  initial #1 $display("r=%b", r);
  initial #5 $finish;
endmodule
