`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  reg [3:0] x, m2;
  function [3:0] inside; input [3:0] v; inside = v + 4'd1; endfunction
  initial begin
    x = 4'd4;
    case (x) inside(3): m2 = 1; default: m2 = 0; endcase
    $display("G m2=%0d", m2);
    $finish;
  end
endmodule
