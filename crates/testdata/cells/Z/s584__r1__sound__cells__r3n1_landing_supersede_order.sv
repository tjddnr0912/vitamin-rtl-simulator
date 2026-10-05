module top;
  logic a, y; wire w, v;
  assign #0 w = a;
  assign v = y;
  always_comb begin y = w; $display("C t=%0t w=%b", $time, w); end
  initial a = 1;
  initial #0 $display("Z t=%0t v=%b y=%b", $time, v, y);
  initial #5 $finish;
endmodule
