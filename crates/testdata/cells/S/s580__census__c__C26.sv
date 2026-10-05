`timescale 1ns/1ns
class K; function logic m(input logic [3:0] a); return a inside {4'b1?00}; endfunction endclass
module t;
  logic [3:0] v;
  initial begin K k; k = new; v=4'b1100; $display("C26 %b", k.m(v)); #1 $finish; end
  initial #100 $finish;
endmodule
