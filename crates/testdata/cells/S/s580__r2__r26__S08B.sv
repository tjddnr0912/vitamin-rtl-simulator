`timescale 1ns/1ns
module t;
  logic [(4'bx100 inside {4'b1?00}):0] wb_S08;
  initial begin #1 $display("S08B %0d", $bits(wb_S08)); #1 $finish; end
  initial #100 $finish;
endmodule
