module t; logic [69:0] a, b, v, w; let L = 'z; let g(q) = q | 'x;
  initial begin a = 70'h0; b = {70{1'b1}}; v = 'x ? a : b; w = L; #1 $display("A v=%h w=%h g=%h", v, w, 70'(g(a))); end
  initial #100 $finish;
endmodule
