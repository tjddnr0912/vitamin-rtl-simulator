module child;
  reg [3:0] x; integer m;
  initial begin #1 x = 4'd4; case (x) inside(3): m = 1; default: m = 0; endcase $display("child upward m=%0d", m); end
endmodule
module top;
  function [3:0] inside; input [3:0] a; inside = a + 1; endfunction
  child c();
  initial #10 $finish;
endmodule
