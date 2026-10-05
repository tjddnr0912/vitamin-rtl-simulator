`timescale 1ns/1ns
module t;
  logic [3:0] v; wire [7:0] w8; assign w8 = v inside {4'b1?00};
  initial begin v=4'b1100; #1 $display("C04 %b", w8); #1 $finish; end
  initial #100 $finish;
endmodule
