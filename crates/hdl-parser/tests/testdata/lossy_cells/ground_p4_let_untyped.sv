module t;
  let g(x) = x + 1'b1;
  initial begin #1 $display("A g=%0d", g(5'h1E)); end
  initial #100 $finish;
endmodule
