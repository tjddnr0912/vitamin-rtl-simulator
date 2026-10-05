`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin v=4'b1100; $display("C02 %h", (v inside {4'b1?00}) ? 8'd1 : 8'd2); #1 $finish; end
  initial #100 $finish;
endmodule
