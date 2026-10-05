`timescale 1ns/1ns
module t;
  initial #1 $display("IU_W %b", $isunknown(((4'd15 + 4'd1) ==? 5'b1?000)));
  localparam L = $isunknown(((4'd15 + 4'd1) ==? 5'b1?000));
  initial #1 $display("IUc_W %b", L);
endmodule
