primitive inv_u (o, i); output o; input i; table 0 : 1 ; 1 : 0 ; endtable endprimitive
module top;
  logic k = 1'b0; logic t; wire o;
  inv_u u (o, k);
  always @(t) $display("R t=%0t o=%b", $time, o);
  initial t = 1;
  initial #0 $display("Z t=%0t o=%b", $time, o);
  initial #5 $finish;
endmodule
