`timescale 1ns/1ns
module t;
  localparam logic [3:0] L = ((4'd15 + 4'd1) ==? 5'b1?000);
  initial #1 $display("LT_W %b", L);
endmodule
