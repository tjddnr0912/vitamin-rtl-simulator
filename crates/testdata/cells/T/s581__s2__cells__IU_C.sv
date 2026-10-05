`timescale 1ns/1ns
module t;
  initial #1 $display("IU_C %b", $isunknown((4'b1100 == 4'b1100)));
  localparam L = $isunknown((4'b1100 == 4'b1100));
  initial #1 $display("IUc_C %b", L);
endmodule
