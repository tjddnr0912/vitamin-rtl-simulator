module top;
  function [3:0] inside; input [3:0] a; inside = a + 1; endfunction
  child c();
  initial #10 $finish;
endmodule
