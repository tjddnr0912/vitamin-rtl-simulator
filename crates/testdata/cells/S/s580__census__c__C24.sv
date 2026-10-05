`timescale 1ns/1ns
module t;
  logic [3:0] v; logic clk = 0; always #1 clk = ~clk;
  assert property (@(posedge clk) (v inside {4'b1?00}) |-> ##1 (v inside {4'b1?00})) else $display("C24 fail %0t", $time);
  initial begin v=4'b1100; #6 $display("C24 end"); $finish; end
  initial #100 $finish;
endmodule
