`timescale 1ns/1ns
module t;
  initial #1 $display("BI_X %0d", $bits(logic [(4'bx100 ==? 4'b1?00):0]));
endmodule
