module t; logic [69:0] b; let LW = 40;
  initial begin b = '0; b[LW-1:0] = 'x; #1 $display("A b=%h", b); end
  initial #100 $finish;
endmodule
