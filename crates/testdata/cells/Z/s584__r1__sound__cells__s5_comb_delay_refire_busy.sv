module top;
  logic a; int n;
  always_comb begin n = n + 1; $display("C t=%0t a=%b n=%0d", $time, a, n); #2; $display("D t=%0t n=%0d", $time, n); end
  initial begin #1 a = 1; #5 $finish; end
endmodule
