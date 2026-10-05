`timescale 1ns/1ns
module child(output reg [3:0] m);
  reg [3:0] x;
  initial begin x = 4'd4; case (x) inside(3): m = 1; default: m = 0; endcase end
endmodule
module t;
  initial #1000 $finish; // watchdog
  function [3:0] inside; input [3:0] v; inside = v + 4'd1; endfunction
  wire [3:0] r;
  child u(r);
  initial begin #1 $display("up m=%0d", r); $finish; end
endmodule
