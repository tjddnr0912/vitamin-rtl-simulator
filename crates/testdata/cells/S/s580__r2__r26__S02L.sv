`timescale 1ns/1ns
module t;
  localparam LP_S02 = (4'b1100 inside {4'sb1?00});
  initial begin #1 $display("S02L %b", LP_S02); #1 $finish; end
  initial #100 $finish;
endmodule
