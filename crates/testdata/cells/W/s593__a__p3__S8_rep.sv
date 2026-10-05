`timescale 1ns/1ns
module t;
  wire [7:0] r = {((8'sb1111_1100 ==? 4'sb1?00)+1){4'b1010}};
  initial #1 $display("r=%b", r);
  initial #5 $finish;
endmodule
