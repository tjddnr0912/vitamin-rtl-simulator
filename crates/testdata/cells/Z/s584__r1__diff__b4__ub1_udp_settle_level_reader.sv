primitive inv(o, a); output o; input a; table 0:1; 1:0; endtable endprimitive
module top;
  wire w = 1'b1;
  wire o;
  logic r;
  inv u(o, w);
  always @(w) r = o;
  initial #1 begin $display("t=%0t o=%b r=%b", $time, o, r); $finish; end
endmodule
