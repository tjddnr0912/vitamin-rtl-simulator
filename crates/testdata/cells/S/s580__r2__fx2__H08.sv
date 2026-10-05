`timescale 1ns/1ns
module t;
  localparam bit S = 1'b1;
  case (S)
    (4'bx100 ==? 4'b1?00): begin : gx initial $display("XB x-label"); end
    1'b1: begin : g1 initial $display("XB one"); end
    default: begin : gd initial $display("XB default"); end
  endcase
  case (S)
    (4'bx100 inside {4'b1?00}): begin : hx initial $display("XI x-label"); end
    1'b1: begin : h1 initial $display("XI one"); end
    default: begin : hd initial $display("XI default"); end
  endcase
  case (S)
    (4'bx100 ==? 4'b1?00): begin : kx initial $display("XD x-label"); end
    default: begin : kd initial $display("XD default"); end
  endcase
  case (S)
    (4'b1100 inside {4'b1?00}): begin : fx initial $display("FOLD item"); end
    default: begin : fd initial $display("FOLD default"); end
  endcase
  initial #1 $finish;
endmodule
