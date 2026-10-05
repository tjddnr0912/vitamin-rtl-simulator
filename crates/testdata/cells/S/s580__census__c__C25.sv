`timescale 1ns/1ns
package p; function automatic logic pf(input logic [3:0] a); return a inside {4'b1?00}; endfunction endpackage
module t;
  logic [3:0] v;
  initial begin v=4'b1100; $display("C25 %b", p::pf(v)); #1 $finish; end
  initial #100 $finish;
endmodule
