`timescale 1ns/1ns
module t;
  logic [3:0] v; logic clk = 0;
  always #1 clk = ~clk;
  assert property (@(posedge clk) v inside {4'b1?00}) else $display("property fail %0t", $time);
  initial begin v = 4'b1100; #6 $display("property end"); $finish; end
endmodule
