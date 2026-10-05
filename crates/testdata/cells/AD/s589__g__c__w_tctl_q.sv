package q;
  typedef logic [3:0] T;
  function automatic T h(input int x); return x; endfunction
endpackage
module top;
  initial begin #1 $display("B=%0d", $bits(q::h(0))); $finish; end
endmodule
