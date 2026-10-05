module top;
  logic s, u, k;
  always_comb s = ~k;
  always @(s) $display("R t=%0t s=%b u=%b", $time, s, u);
  initial begin k = 0; #0 u = 1; end
  initial #5 $finish;
endmodule
