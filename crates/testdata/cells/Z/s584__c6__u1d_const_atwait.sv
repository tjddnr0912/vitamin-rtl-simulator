primitive inv(o, a);
  output o; input a;
  table 0:1; 1:0; endtable
endprimitive
module top;
  wire o;
  int n = 0;
  inv u(o, 1'b0);
  always @(o) begin n++; $display("w t=%0t o=%b", $time, o); end
  initial #1 begin $display("e t=%0t o=%b n=%0d", $time, o, n); $finish; end
endmodule
