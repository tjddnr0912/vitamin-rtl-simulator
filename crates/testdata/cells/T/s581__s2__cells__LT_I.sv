`timescale 1ns/1ns
module t;
  localparam logic [3:0] L = (4'b1100 inside {4'b1?00});
  initial #1 $display("LT_I %b", L);
endmodule
