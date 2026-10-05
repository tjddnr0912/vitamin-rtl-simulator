`timescale 1ns/1ns
module t;
  initial #1 $display("IU_X %b", $isunknown((4'bx100 ==? 4'b1?00)));
  localparam L = $isunknown((4'bx100 ==? 4'b1?00));
  initial #1 $display("IUc_X %b", L);
endmodule
