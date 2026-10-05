`timescale 1ns/1ns
module t;
  string s = "ab";
  initial #1 $display("NW2 %b", s ==? 'x);
endmodule
