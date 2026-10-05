module top;
  logic [1:0] s = 2'd1, k = 2'd2;
  logic [1:0] a, x, b, d;
  always_comb a = s;
  always_comb x = k ^ 2'd3;
  always_comb begin b = a | x; $display("B t=%0t a=%b x=%b", $time, a, x); end
  always_comb begin d = b; $display("D t=%0t b=%b", $time, b); end
  initial #1 $display("t=%0t d=%b", $time, d);
  initial #5 $finish;
endmodule
