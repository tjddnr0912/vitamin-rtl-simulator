`timescale 1ns/1ns
module t;
  localparam int N = 2;
  logic [15:0] v = 16'hABCE;
  initial #1 $display("pw=%0d", v[0 +: ({N{1'b0}} + 2'd2)]);
  initial #50 $finish;
endmodule
