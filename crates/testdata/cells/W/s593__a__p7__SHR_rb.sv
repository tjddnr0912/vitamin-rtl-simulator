`timescale 1ns/1ns
module t;
  localparam logic signed [3:0] SA = -4'sd2;
  logic [((SA >>> 1) ==? 4'sb111?)+3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
