primitive inv(o, a); output o; input a; table 0:1; 1:0; endtable endprimitive
module top;
  wire o;
  logic k = 1'b0;
  logic t;
  inv u(o, k);
  initial t = 1'b1;
  always @(t) $display("b2 t=%0t o=%b", $time, o);
  initial #1 begin $display("e t=%0t o=%b", $time, o); $finish; end
endmodule
