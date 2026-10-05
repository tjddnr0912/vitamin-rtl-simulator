primitive inv(o, a); output o; input a; table 0:1; 1:0; endtable endprimitive
module top;
  logic a = 1'b1;
  logic c, z;
  wire u_o;
  inv u(u_o, c);
  always_comb c = ~a;
  always_comb z = u_o & a;
  logic t;
  initial t = 1'b1;
  always @(t) $display("b2 t=%0t c=%b u_o=%b z=%b", $time, c, u_o, z);
  initial #1 begin $display("e t=%0t c=%b u_o=%b z=%b", $time, c, u_o, z); $finish; end
endmodule
