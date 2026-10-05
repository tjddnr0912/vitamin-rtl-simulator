module top;
  logic [1:0] y;
  always_comb begin y = 0; unique case (2'd3) 2'd1: y = 1; endcase end
  initial #1 $finish;
endmodule
