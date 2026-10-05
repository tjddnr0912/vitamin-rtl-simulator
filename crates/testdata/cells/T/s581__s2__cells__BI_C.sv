`timescale 1ns/1ns
module t;
  initial #1 $display("BI_C %0d", $bits(logic [(4'b1100 == 4'b1100):0]));
endmodule
