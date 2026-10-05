`timescale 1ns/1ns
module t;
  logic [((4'd15 + 4'd1) ==? 5'b1?000) : 0] v;
  initial #1 $display("RB_W %0d", $bits(v));
endmodule
