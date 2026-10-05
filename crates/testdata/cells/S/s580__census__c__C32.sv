`timescale 1ns/1ns
module t;
  logic [3:0] v; logic [3:0] q [$];
  initial begin v=4'b1100; q.push_back(4'b1100); $display("C32 %b", q[0] inside {4'b1?00}); #1 $finish; end
  initial #100 $finish;
endmodule
