`timescale 1ns/1ns
module t;
  for (genvar i = 0; i <= ((4'd15 + 4'd1) inside {5'b0?000}); i++) begin : g initial #1 $display("GF_N %0d", i); end
endmodule
