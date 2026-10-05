primitive inv(o, a); output o; input a; table 0:1; 1:0; endtable endprimitive
module top;
  logic k = 1'b0;
  wire o;
  inv #2 u(o, k);
  always @(o) $display("w t=%0t o=%b", $time, o);
  initial begin #1 $display("a t=%0t o=%b", $time, o); #4 k = 1'b1; #1 $display("b t=%0t o=%b", $time, o); #2 $display("c t=%0t o=%b", $time, o); $finish; end
endmodule
