`timescale 1ns/1ns
module t;
  initial #1 $display("IU_I %b", $isunknown((4'b1100 inside {4'b1?00})));
  localparam L = $isunknown((4'b1100 inside {4'b1?00}));
  initial #1 $display("IUc_I %b", L);
endmodule
