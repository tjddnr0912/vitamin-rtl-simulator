`timescale 1ns/1ns
module gfun(output reg [3:0] m);
  reg [3:0] x;
  generate if (1) begin : g
    function [3:0] inside; input [3:0] v; inside = v + 4'd1; endfunction
    initial begin x = 4'd4; case (x) inside(3): m = 1; default: m = 0; endcase end
  end endgenerate
endmodule
module afun(output [3:0] m);
  function [3:0] f1; input [3:0] v; input [3:0] inside;
    begin case (v) inside[2:1]: f1 = 1; default: f1 = 0; endcase end
  endfunction
  assign m = f1(4'd3, 4'b0110);
endmodule
module cfun(output reg [3:0] m);
  reg [3:0] a;
  function [3:0] f2; input [3:0] v; input [3:0] inside;
    begin case (v) inside + 1: f2 = 1; default: f2 = 0; endcase end
  endfunction
  always @* m = f2(a, 4'd6);
  initial a = 4'd7;
endmodule
module top;
  wire [3:0] r1, r2, r3;
  gfun u1(r1); afun u2(r2); cfun u3(r3);
  initial begin #1 $display("gfun=%0d afun=%0d cfun=%0d", r1, r2, r3); $finish; end
  initial #1000 $finish;
endmodule
