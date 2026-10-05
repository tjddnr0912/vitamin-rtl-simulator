`timescale 1ns/1ns
module t;
  localparam [64:0] L = {64'd0, (4'b1100 inside {4'b1?00})};
  initial #1 $display("LW_I %h", L);
endmodule
