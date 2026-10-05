package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x); logic [f(2):0] t; t = x; return t; endfunction
endpackage
module top;
  import q::h;
  localparam int P = h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
