`timescale 1ns/1ns
module t;
  for (genvar i = 0; i <= ((4'd15 + 4'd1) ==? 5'b1?000); i++) begin : g initial #1 $display("GF_W %0d", i); end
endmodule
