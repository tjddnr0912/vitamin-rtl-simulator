`timescale 1ns/1ns
module t;
  logic [3:0] v; wire [3:0] y; assign y = (v inside {4'b1?00}) ? 4'd1 : 4'd2;
  initial begin v=4'b1100; #1 $display("C23 %h", y); #1 $finish; end
  initial #100 $finish;
endmodule
