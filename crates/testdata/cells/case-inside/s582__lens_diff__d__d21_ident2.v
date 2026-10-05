`timescale 1ns/1ns
module fsub;
  reg [3:0] x, m2;
  function [3:0] inside; input [3:0] v; inside = v + 4'd1; endfunction
  initial begin
    x = 4'd4;
    case (x)
      inside(3): m2 = 1;
      default: m2 = 0;
    endcase
    $display("G m2=%0d", m2);
  end
endmodule
module top;
  reg [3:0] x, inside;
  reg [3:0] m1;
  fsub u();
  initial begin
    inside = 4'b0110; x = 4'd3;
    casez (x)
      inside[2:1]: m1 = 1;
      default: m1 = 0;
    endcase
    #1 $display("F m1=%0d", m1);
    $finish;
  end
  initial #1000 $finish;
endmodule
