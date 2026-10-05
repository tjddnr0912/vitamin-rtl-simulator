`timescale 1ns/1ns
module t;
  logic [3:0] v; logic [3:0] r;
  always @* r = (v inside {4'b1?00}) ? 4'd7 : 4'd3;
  initial begin v = 4'b1000; #1 $display("Z07 %d", r); v = 4'b0000; #1 $display("Z07 %d", r); #1 $finish; end
  initial #100 $finish;
endmodule
