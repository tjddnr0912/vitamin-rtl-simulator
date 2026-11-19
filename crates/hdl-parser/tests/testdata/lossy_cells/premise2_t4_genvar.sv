module t;
  for (genvar g = 'x; g < 2; g = g + 1) begin : gl initial $display("A g=%0d", g); end
  initial #100 $finish;
endmodule
