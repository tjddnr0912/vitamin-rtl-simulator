module top;
  logic a = 0, b, c;
  always_comb begin $display("P1"); b = a; end
  always_comb begin $display("P2"); c = b; end
  always_comb begin $display("P3 abc=%b%b%b", a, b, c); end
  initial begin $display("I"); #0 $display("I#0"); end
  initial #2 $finish;
  initial #100 $finish;
endmodule
