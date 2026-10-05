module top;
  logic a, y;
  always_comb begin y = a; $display("C t=%0t a=%b", $time, a); if (a) $finish; end
  initial a = 1;
endmodule
