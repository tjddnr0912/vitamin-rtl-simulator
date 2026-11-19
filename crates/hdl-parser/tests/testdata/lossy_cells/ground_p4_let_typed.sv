module t;
  let f(logic [3:0] x) = x + 1'b1;
  initial begin #1 $display("A f=%0d", f(5'h1E)); end
  initial #100 $finish;
endmodule
