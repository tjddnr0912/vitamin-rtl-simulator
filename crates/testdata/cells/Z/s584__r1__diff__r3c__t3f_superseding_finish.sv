module top;
  logic [1:0] src = 2'd1;
  logic [1:0] a, b, c;
  always_comb begin a = src; $display("A t=%0t", $time); end
  always_comb begin b = a; $display("B t=%0t a=%b", $time, a); if (a == 2'd1) $finish; end
  always_comb begin c = b; $display("C t=%0t b=%b", $time, b); end
  initial #5 $display("late");
endmodule
