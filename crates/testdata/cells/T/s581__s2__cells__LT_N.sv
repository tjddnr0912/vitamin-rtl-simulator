`timescale 1ns/1ns
module t;
  localparam logic [3:0] L = ((4'd15 + 4'd1) inside {5'b0?000});
  initial #1 $display("LT_N %b", L);
endmodule
