module top;
  logic p, q, r;
  always_comb begin q = p; $display("A t=%0t p=%b", $time, p); end
  always_comb begin r = q; $display("B t=%0t q=%b", $time, q); end
  initial #5 $finish;
endmodule
