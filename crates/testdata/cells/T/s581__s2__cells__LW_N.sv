`timescale 1ns/1ns
module t;
  localparam [64:0] L = {64'd0, ((4'd15 + 4'd1) inside {5'b0?000})};
  initial #1 $display("LW_N %h", L);
endmodule
