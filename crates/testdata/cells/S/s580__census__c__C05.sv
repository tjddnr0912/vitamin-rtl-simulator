`timescale 1ns/1ns
module t;
  logic [3:0] v;
  function automatic logic f(input logic [3:0] a); return a inside {4'b1?00}; endfunction
  initial begin v=4'b1100; $display("C05 %b", f(v)); #1 $finish; end
  initial #100 $finish;
endmodule
