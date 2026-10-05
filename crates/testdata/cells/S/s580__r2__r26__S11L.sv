`timescale 1ns/1ns
module t;
  localparam LP_S11 = ((4'd15 + 4'd1) inside {5'b1?000});
  initial begin #1 $display("S11L %b", LP_S11); #1 $finish; end
  initial #100 $finish;
endmodule
