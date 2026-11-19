module t; logic [127:0] v, w; logic c;
  initial begin w='1; c=1; v = c ? 'x : w; #1 $display("A v=%h", v); c=0; v = c ? w : 'z; #1 $display("B v=%h", v); end
  initial #100 $finish;
endmodule
