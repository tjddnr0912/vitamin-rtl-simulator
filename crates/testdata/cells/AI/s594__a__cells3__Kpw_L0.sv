`timescale 1ns/1ns
module t;
  logic [15:0] v = 16'hABCE;
  initial #1 $display("pw=%0d", v[0 +: (2'b00 + 2'd2)]);
  initial #40 $finish;
endmodule
