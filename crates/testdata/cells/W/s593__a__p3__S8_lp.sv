`timescale 1ns/1ns
module t;
  localparam L = (8'sb1111_1100 ==? 4'sb1?00);
  initial $display("L=%b", L);
  initial #5 $finish;
endmodule
