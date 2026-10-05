`timescale 1ns/1ns
module t;
  initial #1 $display("BI_W %0d", $bits(logic [((4'd15 + 4'd1) ==? 5'b1?000):0]));
endmodule
