module t; logic clk = 0; always #5 clk = ~clk; int np, nf;
  property p; logic [69:0] lv = 'x; @(posedge clk) (lv === 70'bx); endproperty
  sequence s; logic [69:0] sv = 'z; @(posedge clk) (sv === 70'bz); endsequence
  assert property (p) np++; else nf++;
  cover property (s) $display("C cover hit");
  initial begin np = 0; nf = 0; #22 $display("A np=%0d nf=%0d", np, nf); end
  initial #30 $finish;
endmodule
