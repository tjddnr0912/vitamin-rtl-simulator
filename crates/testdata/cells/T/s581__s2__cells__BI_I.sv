`timescale 1ns/1ns
module t;
  initial #1 $display("BI_I %0d", $bits(logic [(4'b1100 inside {4'b1?00}):0]));
endmodule
