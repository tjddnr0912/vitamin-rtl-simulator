`timescale 1ns/1ns
module t;
  localparam logic signed [3:0] SA = -4'sd2;
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: ((SA >>> 1) ==? 4'sb111?)+3];
  initial #1 $display("ps=%b", ps);
  initial #5 $finish;
endmodule
