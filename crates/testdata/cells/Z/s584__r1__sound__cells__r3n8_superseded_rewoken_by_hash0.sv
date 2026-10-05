module top;
  logic a, b, y; wire w;
  assign #0 w = a;
  always_comb begin y = w | b; $display("C t=%0t w=%b b=%b", $time, w, b); end
  initial a = 1;
  initial #0 b = 1;
  initial #5 $display("F y=%b", y);
  initial #6 $finish;
endmodule
