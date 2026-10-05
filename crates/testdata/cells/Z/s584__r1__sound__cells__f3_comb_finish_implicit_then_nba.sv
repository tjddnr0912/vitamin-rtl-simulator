module top;
  logic a, b, y;
  always_comb begin y = a; $display("C t=%0t a=%b", $time, a); if (!a) $finish; end
  initial begin a = 0; b <= 1; end
  always @(b) a = b;
endmodule
