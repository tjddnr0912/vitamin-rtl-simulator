`timescale 1ns/1ns
module t;
  logic [7:0] r;
  initial begin r = {((4'b1100 inside {4'b1?00}) + 1){1'b1}}; #1 $display("RP_I %b", r); end
endmodule
