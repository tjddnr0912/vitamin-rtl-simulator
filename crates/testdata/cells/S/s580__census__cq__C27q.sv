`timescale 1ns/1ns
module t;
  localparam logic L4 = (4'bx100 ==? 4'b1?00);
  initial begin $display("C27q %b", L4); #1 $finish; end
  initial #100 $finish;
endmodule
