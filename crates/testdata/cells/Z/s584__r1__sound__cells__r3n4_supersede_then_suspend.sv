module top;
  logic a, y; wire w; int n;
  assign #0 w = a;
  always_comb begin n = n + 1; $display("C t=%0t w=%b n=%0d", $time, w, n); #1; $display("D t=%0t n=%0d", $time, n); end
  initial begin a = 1; #5 $finish; end
endmodule
