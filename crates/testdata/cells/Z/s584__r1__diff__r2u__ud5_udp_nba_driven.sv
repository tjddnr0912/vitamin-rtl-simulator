primitive inv(o, a); output o; input a; table 0:1; 1:0; endtable endprimitive
module top;
  logic k;
  wire o;
  inv u(o, k);
  initial k <= 1'b0;
  always @(o) $display("w t=%0t o=%b", $time, o);
  initial begin #0 $display("z t=%0t o=%b", $time, o); $strobe("s t=%0t o=%b", $time, o); #1 $display("e t=%0t o=%b", $time, o); $finish; end
endmodule
