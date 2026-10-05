module top; logic a, b; logic [1:0] y;
  always @(a or b) begin unique if (a) y = 1; else if (b) y = 2; end
  initial begin a = 1; b = 0; #1 a = 0; #1 b = 1; #1 b = 0; #1 $finish; end
endmodule
