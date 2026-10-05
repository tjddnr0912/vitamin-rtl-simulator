package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  function automatic int f(input int a); f = 7; if (a == 1) f = 10; endfunction
  localparam P = q::h(1000);
  initial begin #1 $display("P=%0d b=%0d", P, $bits(P)); $finish; end
endmodule
