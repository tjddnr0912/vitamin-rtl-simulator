primitive inv(o, a);
  output o; input a;
  table 0:1; 1:0; endtable
endprimitive
module top;
  wire o;
  inv u(o, 1'b0);
  initial #0 $display("z t=%0t o=%b", $time, o);
  initial #1 begin $display("e t=%0t o=%b", $time, o); $finish; end
endmodule
