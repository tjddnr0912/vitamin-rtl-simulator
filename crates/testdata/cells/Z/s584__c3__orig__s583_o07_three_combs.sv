module top;
  logic a = 0, b, c;
  always_comb begin $display("P1 t=%0t", $time); b = a; end
  always_comb begin $display("P2 t=%0t", $time); c = b; end
  always_comb begin $display("P3 t=%0t abc=%b%b%b", $time, a, b, c); end
  initial begin $display("I t=%0t", $time); #0 $display("I#0"); end
  always @(a) $display("AT t=%0t", $time);
  initial #2 $finish;
endmodule
