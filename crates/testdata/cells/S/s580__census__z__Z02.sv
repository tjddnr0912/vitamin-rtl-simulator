`timescale 1ns/1ns
module c; logic signed [7:0] sig = -8'sd4; endmodule
module t;
  c u();
  initial begin #1 $display("Z02 %b", u.sig inside {4'sb1?00}); #1 $finish; end
  initial #100 $finish;
endmodule
