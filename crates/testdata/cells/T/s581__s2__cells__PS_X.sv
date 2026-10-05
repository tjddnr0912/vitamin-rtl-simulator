`timescale 1ns/1ns
module t;
  logic [7:0] v8 = 8'hA5;
  initial #1 $display("PS_X %b", v8[(4'bx100 ==? 4'b1?00)*3+1:0]);
endmodule
