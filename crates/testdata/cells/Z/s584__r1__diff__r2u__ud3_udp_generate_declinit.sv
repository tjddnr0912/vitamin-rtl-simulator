primitive inv(o, a); output o; input a; table 0:1; 1:0; endtable endprimitive
module top;
  logic [3:0] k = 4'b1010;
  wire [3:0] o;
  logic t;
  genvar i;
  for (i = 0; i < 4; i++) begin : g
    inv u(o[i], k[i]);
  end
  initial t = 1'b1;
  always @(t) $display("b2 t=%0t o=%b", $time, o);
  initial #1 begin $display("e t=%0t o=%b", $time, o); $finish; end
endmodule
