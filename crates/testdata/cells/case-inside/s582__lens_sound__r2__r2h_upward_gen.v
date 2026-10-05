module child2;
  reg [3:0] x; integer m;
  initial begin #1 x = 4'd4; case (x) inside(3) + 4'd0, 4'd9: m = 1; default: m = 0; endcase $display("child2 upward-gen m=%0d", m); end
endmodule
module top;
  generate if (1) begin : g
    function [3:0] inside; input [3:0] a; inside = a + 1; endfunction
    child2 c();
  end endgenerate
  initial #10 $finish;
endmodule
