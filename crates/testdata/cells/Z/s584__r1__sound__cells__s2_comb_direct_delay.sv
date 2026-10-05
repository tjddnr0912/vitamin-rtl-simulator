module top;
  logic a, y; int n;
  always_comb begin n = n + 1; y = a; $display("C t=%0t a=%b n=%0d", $time, a, n); #1; $display("D t=%0t n=%0d", $time, n); end
  initial begin a = 0; #5 a = 1; #5 $finish; end
endmodule
