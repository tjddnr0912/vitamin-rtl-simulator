primitive inv(o, a); output o; input a; table 0:1; 1:0; endtable endprimitive
module ch(output wire o);
  logic k = 1'b1;
  inv u(o, k);
endmodule
module top;
  wire o;
  logic t;
  ch c(.o(o));
  initial t = 1'b1;
  always @(t) $display("b2 t=%0t o=%b", $time, o);
  initial #1 begin $display("e t=%0t o=%b", $time, o); $finish; end
endmodule
