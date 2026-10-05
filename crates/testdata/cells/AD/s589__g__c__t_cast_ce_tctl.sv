package q;
  typedef logic [3:0] T;
  function automatic int h(input int x); return T'(x); endfunction
endpackage
module top;
  localparam int P = q::h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
