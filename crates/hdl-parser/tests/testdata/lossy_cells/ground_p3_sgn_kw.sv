module t;
  parameter signed N = 4'hF;
  initial begin #1 $display("A n=%0d", N); end
  initial #100 $finish;
endmodule
