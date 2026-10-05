`timescale 1ns/1ns
module t;
  localparam logic L1 = (4'b1100 inside {4'b1?00});
  initial begin $display("C10 %b", L1); #1 $finish; end
  initial #100 $finish;
endmodule
