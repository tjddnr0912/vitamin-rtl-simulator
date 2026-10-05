`timescale 1ns/1ns
module t;
  logic [3:0] v; wire w; assign w = v inside {4'b1?00};
  initial begin v=4'b1100; #1 $display("C03 %b", w); #1 $finish; end
  initial #100 $finish;
endmodule
