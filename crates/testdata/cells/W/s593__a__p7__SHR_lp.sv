`timescale 1ns/1ns
module t;
  localparam logic signed [3:0] SA = -4'sd2;
  localparam L = ((SA >>> 1) ==? 4'sb111?);
  initial #1 $display("L=%b", L);
  initial #5 $finish;
endmodule
