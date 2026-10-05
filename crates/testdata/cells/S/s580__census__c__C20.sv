`timescale 1ns/1ns
module t;
  logic [3:0] v; logic m;
  always_comb m = v inside {4'b1?00};
  initial begin v=4'b1100; #1 $display("C20 %b", m); #1 $finish; end
  initial #100 $finish;
endmodule
