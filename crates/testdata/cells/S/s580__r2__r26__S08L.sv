`timescale 1ns/1ns
module t;
  localparam LP_S08 = (4'bx100 inside {4'b1?00});
  initial begin #1 $display("S08L %b", LP_S08); #1 $finish; end
  initial #100 $finish;
endmodule
