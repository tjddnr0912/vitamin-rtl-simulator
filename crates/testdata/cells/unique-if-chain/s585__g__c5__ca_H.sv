module top;
  logic clk = 0, a, b, en; logic [1:0] y, q, l;
  always_ff @(posedge clk) begin if (a) q <= 1; else unique if (b) q <= 2; end
  always_latch begin if (en) begin if (a) l = 1; else unique if (b) l = 2; end end
  initial begin a = 0; b = 0; en = 0; #1 clk = 1; #1 en = 1; #1 a = 1; #1 $finish; end
endmodule
