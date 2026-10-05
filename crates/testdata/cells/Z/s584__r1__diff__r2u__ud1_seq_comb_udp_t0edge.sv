primitive inv(o, a); output o; input a; table 0:1; 1:0; endtable endprimitive
primitive dff_udp (q, clk, d);
  output q; reg q; input clk, d;
  initial q = 1'b0;
  table
    (01) 0 : ? : 0 ;
    (01) 1 : ? : 1 ;
    (0x) 1 : 1 : 1 ;
    (0x) 0 : 0 : 0 ;
    (?0) ? : ? : - ;
    (x1) ? : ? : - ;
    (1x) ? : ? : - ;
     ? (??) : ? : - ;
  endtable
endprimitive
module top;
  logic k = 1'b0;
  logic clk = 1'b0;
  wire d, q;
  inv u1(d, k);
  dff_udp f(q, clk, d);
  logic t;
  always @(t) $display("b2 t=%0t d=%b q=%b", $time, d, q);
  initial begin t = 1'b1; clk = 1'b1; #1 $display("e t=%0t d=%b q=%b", $time, d, q); $finish; end
endmodule
