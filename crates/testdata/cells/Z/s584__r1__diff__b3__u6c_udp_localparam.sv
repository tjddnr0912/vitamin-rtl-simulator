primitive inv(o, a); output o; input a; table 0:1; 1:0; endtable endprimitive
module top;
  localparam logic P = 1'b1;
  wire o;
  logic t;
  inv u(o, P);
  initial t = 1'b1;
  always @(t) $display("b2 t=%0t o=%b", $time, o);
  initial #1 begin $display("e t=%0t o=%b", $time, o); $finish; end
endmodule
