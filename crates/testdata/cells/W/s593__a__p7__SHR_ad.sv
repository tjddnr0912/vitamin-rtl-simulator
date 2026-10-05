`timescale 1ns/1ns
module t;
  localparam logic signed [3:0] SA = -4'sd2;
  logic [3:0] arr [0:((SA >>> 1) ==? 4'sb111?)+2];
  initial #1 $display("asz=%0d", $size(arr));
  initial #5 $finish;
endmodule
