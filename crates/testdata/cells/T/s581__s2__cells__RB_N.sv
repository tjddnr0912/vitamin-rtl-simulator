`timescale 1ns/1ns
module t;
  logic [((4'd15 + 4'd1) inside {5'b0?000}) : 0] v;
  initial #1 $display("RB_N %0d", $bits(v));
endmodule
