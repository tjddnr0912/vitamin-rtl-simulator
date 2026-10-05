`timescale 1ns/1ns
module t;
  logic [7:0] r;
  initial begin r = {((4'bx100 ==? 4'b1?00) + 1){1'b1}}; #1 $display("RP_X %b", r); end
endmodule
