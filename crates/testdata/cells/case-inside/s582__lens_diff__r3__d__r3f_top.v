`timescale 1ns/1ns
module top;
  function [3:0] inside; input [3:0] v; inside = v + 4'd1; endfunction
  wire [3:0] r;
  child u(r);
  initial begin #1 $display("up2 m=%0d", r); $finish; end
  initial #1000 $finish;
endmodule
