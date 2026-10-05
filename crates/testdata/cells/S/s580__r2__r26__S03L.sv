`timescale 1ns/1ns
module t;
  localparam LP_S03 = (4'b1100 inside {'b1?00});
  initial begin #1 $display("S03L %b", LP_S03); #1 $finish; end
  initial #100 $finish;
endmodule
