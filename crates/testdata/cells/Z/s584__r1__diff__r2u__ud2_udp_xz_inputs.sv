primitive mux_udp(o, a, b, s);
  output o; input a, b, s;
  table
     0 ? 0 : 0;
     1 ? 0 : 1;
     ? 0 1 : 0;
     ? 1 1 : 1;
     0 0 x : 0;
     1 1 x : 1;
  endtable
endprimitive
module top;
  logic a = 1'b1, b = 1'b1;
  logic s;
  wire sz;
  wire o1, o2;
  mux_udp m1(o1, a, b, s);
  mux_udp m2(o2, a, b, sz);
  logic t;
  initial t = 1'b1;
  always @(t) $display("b2 t=%0t o1=%b o2=%b", $time, o1, o2);
  initial begin $display("i0 t=%0t o1=%b o2=%b", $time, o1, o2); #1 $display("e t=%0t o1=%b o2=%b", $time, o1, o2); $finish; end
endmodule
