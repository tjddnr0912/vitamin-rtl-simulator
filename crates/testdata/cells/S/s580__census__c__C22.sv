`timescale 1ns/1ns
module t;
  logic [3:0] v; integer n;
  initial begin n=0; for (v=4'b1000; v inside {4'b1?0?}; v = v + 4'd1) n = n + 1; $display("C22 %0d", n); #1 $finish; end
  initial #100 $finish;
endmodule
