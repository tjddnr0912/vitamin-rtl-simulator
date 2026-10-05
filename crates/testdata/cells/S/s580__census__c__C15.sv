`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin v=4'b0000; fork begin wait (v inside {4'b1?00}); $display("C15 woke %0t", $time); end join_none #1 v=4'b1000; #2 $display("C15 end"); $finish; end
  initial #100 $finish;
endmodule
