`timescale 1ns/1ns
module t;
  for (genvar g = 0; g < (2'b11 + 2'd2); g = g + 1) begin : gl initial #1 $display("g=%0d", g); end
  initial #20 $finish;
endmodule
