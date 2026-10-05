primitive inv_u (o, i); output o; input i; table 0 : 1 ; 1 : 0 ; endtable endprimitive
module top;
  wire w; assign w = 1'b0;
  logic k, t; wire k2, o;
  assign k2 = w ^ k;
  inv_u u (o, k2);
  always @(t) $display("R t=%0t o=%b k2=%b", $time, o, k2);
  always @(o) $display("W t=%0t o=%b", $time, o);
  initial begin k = 1; t = 1; end
  initial #0 $display("Z t=%0t o=%b", $time, o);
  initial #5 $finish;
endmodule
