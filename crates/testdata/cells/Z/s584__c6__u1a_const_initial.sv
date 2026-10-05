primitive inv(o, a);
  output o; input a;
  table 0:1; 1:0; endtable
endprimitive
module top;
  wire o;
  inv u(o, 1'b0);
  initial begin $display("i0 t=%0t o=%b", $time, o); #1 $display("e t=%0t o=%b", $time, o); $finish; end
endmodule
