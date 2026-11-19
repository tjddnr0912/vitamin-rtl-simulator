module t; logic [127:0] v, w;
  initial begin w='0; v = w | 'z; #1 $display("A v=%h", v); v = w ^ 'x; #1 $display("B v=%h eq=%b", v, (v === 'x)); end
  initial #100 $finish;
endmodule
