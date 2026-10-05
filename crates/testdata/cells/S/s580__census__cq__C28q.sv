`timescale 1ns/1ns
module t;
  logic [(4'b1100 ==? 4'b1?00) : 0] w;
  initial begin $display("C28q %0d", $bits(w)); #1 $finish; end
  initial #100 $finish;
endmodule
