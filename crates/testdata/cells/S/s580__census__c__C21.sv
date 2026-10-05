`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin v=4'b1000; while (v inside {4'b1?00}) v = v + 4'd1; $display("C21 %b", v); #1 $finish; end
  initial #100 $finish;
endmodule
