`timescale 1ns/1ns
module t;
  initial #1 $display("IU_N %b", $isunknown(((4'd15 + 4'd1) inside {5'b0?000})));
  localparam L = $isunknown(((4'd15 + 4'd1) inside {5'b0?000}));
  initial #1 $display("IUc_N %b", L);
endmodule
