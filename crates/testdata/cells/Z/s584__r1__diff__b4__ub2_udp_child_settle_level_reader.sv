primitive inv(o, a); output o; input a; table 0:1; 1:0; endtable endprimitive
module ch(input wire i, output wire o);
  inv u(o, i);
endmodule
module top;
  wire w = 1'b0;
  wire o;
  logic r;
  logic t;
  ch c(.i(w), .o(o));
  always @(w) begin r = o; $display("L t=%0t w=%b o=%b", $time, w, o); end
  initial t = 1'b1;
  initial #1 begin $display("t=%0t o=%b r=%b", $time, o, r); $finish; end
endmodule
