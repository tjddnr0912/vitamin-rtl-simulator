`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin v=4'b1100; if (v inside {4'b1?00}) $display("C01 then"); else $display("C01 else"); #1 $finish; end
  initial #100 $finish;
endmodule
