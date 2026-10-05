`timescale 1ns/1ns
module t;
  logic d [(2'b11 + 2'd2):0];
  initial #1 $display("ds=%0d", $size(d));
  initial #20 $finish;
endmodule
