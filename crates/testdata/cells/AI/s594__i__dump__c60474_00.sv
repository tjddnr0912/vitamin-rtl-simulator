`timescale 1ns/1ns
module t;
  localparam int N = 2;
  logic [15:0] v = 16'hABCD;
  initial #1 $display("pw=%h", v[0 +: ({N{1'b0}} + 2'd3)]);
  initial #50 $finish;
endmodule
