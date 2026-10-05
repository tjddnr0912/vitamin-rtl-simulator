`timescale 1ns/1ns
module t;
  logic [15:0] v = 16'hABCD;
  initial #1 $display("pw=%h", v[0 +: (2'b00 + 2'd3)]);
  initial #40 $finish;
endmodule
