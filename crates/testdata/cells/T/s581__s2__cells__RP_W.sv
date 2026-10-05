`timescale 1ns/1ns
module t;
  logic [7:0] r;
  initial begin r = {(((4'd15 + 4'd1) ==? 5'b1?000) + 1){1'b1}}; #1 $display("RP_W %b", r); end
endmodule
