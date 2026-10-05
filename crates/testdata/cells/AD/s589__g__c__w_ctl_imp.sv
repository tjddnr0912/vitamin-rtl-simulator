package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [f(2):0] h(input int x); return x; endfunction
endpackage
module top;
  import q::h;
  initial begin #1 $display("B=%0d", $bits(h(0))); $finish; end
endmodule
