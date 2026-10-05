`timescale 1ns/1ns
class K; int a; endclass
module t;
  initial begin K h, h2; h = new; h2 = h; $display("Z03 %b %b", h inside {null, h2}, h2 inside {null}); #1 $finish; end
  initial #100 $finish;
endmodule
