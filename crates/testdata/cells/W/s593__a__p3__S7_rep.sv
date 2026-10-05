`timescale 1ns/1ns
module t;
  wire [7:0] r = {(((4'd15 + 4'd1) ==? 5'b1?000)+1){4'b1010}};
  initial #1 $display("r=%b", r);
  initial #5 $finish;
endmodule
