package q;
  localparam int W = 4;
  function automatic logic [W-1:0] g(input int a); g = '1; endfunction
endpackage
module c #(parameter P = 3) ();
  initial #1 $display("P=%0d b=%0d", P, $bits(P));
endmodule
module top;
  localparam int W = 8;
  c #(.P(q::g(0))) u ();
  initial #2 $finish;
endmodule
