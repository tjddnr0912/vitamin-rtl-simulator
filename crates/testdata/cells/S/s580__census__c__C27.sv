`timescale 1ns/1ns
module t;
  localparam logic L4 = (4'bx100 inside {4'b1?00});
  initial begin $display("C27 %b", L4); #1 $finish; end
  initial #100 $finish;
endmodule
