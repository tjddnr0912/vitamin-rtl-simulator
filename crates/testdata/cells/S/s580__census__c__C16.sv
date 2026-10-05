`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin v=4'b1100; assert (v inside {4'b1?00}) $display("C16 pass"); else $display("C16 fail"); #1 $finish; end
  initial #100 $finish;
endmodule
