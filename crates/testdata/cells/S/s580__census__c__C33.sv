`timescale 1ns/1ns
module t;
  logic [3:0] v; event e;
  initial begin v=4'b1100; @(v inside {4'b1?00}) $display("C33 edge"); end
  initial begin #1 v=4'b0100; #1 $display("C33 end"); $finish; end
  initial #100 $finish;
endmodule
