`timescale 1ns/1ns
module t;
  initial #1 $display("BI_N %0d", $bits(logic [((4'd15 + 4'd1) inside {5'b0?000}):0]));
endmodule
