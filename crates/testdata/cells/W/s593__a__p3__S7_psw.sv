`timescale 1ns/1ns
module t;
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: ((4'd15 + 4'd1) ==? 5'b1?000)+3];
  initial #1 $display("ps=%b", ps);
  initial #5 $finish;
endmodule
