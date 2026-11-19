module t; logic clk = 0; always #5 clk = ~clk; logic [69:0] s, s2;
  clocking cb @(posedge clk); output s; endclocking
  initial begin s = 0; @(cb); cb.s <= 'x; @(cb); #1 $display("A s=%h", s); cb.s <= 'z; @(cb); #1 $display("B s=%h", s); end
  initial #100 $finish;
endmodule
