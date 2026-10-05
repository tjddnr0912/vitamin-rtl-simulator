primitive inv_u (o, i);
  output o; input i;
  table 0 : 1 ; 1 : 0 ; endtable
endprimitive
module top;
  logic a; wire y, z;
  inv_u g1 (y, a);
  assign z = y;
  always @(a) $display("R t=%0t a=%b y=%b z=%b", $time, a, y, z);
  initial begin a = 0; #1 a = 1; #1 $finish; end
endmodule
