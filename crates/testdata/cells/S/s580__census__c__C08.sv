`timescale 1ns/1ns
module t;
  logic [3:0] v; wire w;
  function automatic logic f(input logic [3:0] a); return a inside {4'b1?00}; endfunction
  assign w = f(v);
  initial begin v=4'b1100; #1 $display("C08 %b", w); #1 $finish; end
  initial #100 $finish;
endmodule
