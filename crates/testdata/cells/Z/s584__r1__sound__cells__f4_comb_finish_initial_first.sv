module top;
  logic a, y;
  initial a = 1;
  always_comb begin y = a; $display("C t=%0t a=%b", $time, a); if (a) $finish; end
endmodule
