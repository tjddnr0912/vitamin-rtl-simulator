primitive inv(o, a);
  output o; input a;
  table 0:1; 1:0; endtable
endprimitive
module top;
  wire o;
  logic t;
  int n = 0;
  inv u(o, 1'b0);
  always @(o) begin n++; $display("w t=%0t o=%b", $time, o); end
  initial begin $display("i0 t=%0t o=%b", $time, o); t = 1'b1; end
  always @(t) $display("b2 t=%0t o=%b", $time, o);
  initial #0 $display("z t=%0t o=%b", $time, o);
  initial #1 begin $display("e t=%0t o=%b n=%0d", $time, o, n); $finish; end
endmodule
