`timescale 1ns/1ns
module t;
  localparam logic signed [3:0] SA = -4'sd2;
  wire [7:0] r = {(((SA >>> 1) ==? 4'sb111?)+1){4'b1010}};
  initial #1 $display("r=%b", r);
  initial #5 $finish;
endmodule
