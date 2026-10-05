package p;
  localparam int K = 8;
  function logic [K-1:0] f(); logic [K-1:0] s; s = '1; return s; endfunction
endpackage
module m1(output logic [31:0] o); localparam int K = 4; initial begin #1 o = p::f(); end endmodule
module m2(output logic [31:0] o); localparam int K = 16; initial begin #2 o = p::f(); end endmodule
module top;
  logic [31:0] a, b;
  m1 u1(a); m2 u2(b);
  initial begin #3 $display("a=%0d b=%0d", a, b); $finish; end
  initial #100 $finish;
endmodule
