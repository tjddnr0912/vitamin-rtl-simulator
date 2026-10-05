`timescale 1ns/1ns
module t;
  logic [15:0] v = 16'hFFFF;
  initial #1 $display("ps=%0d", $bits(v[0 +: (2'b11 + 2'd2)]));
  initial #20 $finish;
endmodule
