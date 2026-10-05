primitive inv(o, a); output o; input a; table 0:1; 1:0; endtable endprimitive
module top;
  wire o;
  logic t;
  logic k = 1'b0;
  wire kw = k;
  inv u(o, kw);
  initial t = 1'b1;
  always @(t) $display("b2 t=%0t o=%b", $time, o);
  initial #1 begin $display("e t=%0t o=%b", $time, o); $finish; end
endmodule
