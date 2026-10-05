primitive inv_u (o, i); output o; input i; table 0 : 1 ; 1 : 0 ; endtable endprimitive
primitive and_u (o, a, b); output o; input a, b; table 1 1 : 1 ; 0 ? : 0 ; ? 0 : 0 ; endtable endprimitive
module top;
  logic a, b, t; wire n1, n2;
  inv_u g1 (n1, a);
  and_u g2 (n2, n1, b);
  always @(t) $display("R t=%0t n1=%b n2=%b", $time, n1, n2);
  always @(n2) $display("W t=%0t n2=%b", $time, n2);
  initial begin a = 0; b = 1; t = 1; #1 a = 1; #1 t = 0; #1 $finish; end
endmodule
