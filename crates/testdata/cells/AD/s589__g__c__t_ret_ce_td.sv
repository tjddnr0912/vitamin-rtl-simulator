package q;
  typedef logic [3:0] T;
  function automatic T h(input int x); return x; endfunction
endpackage
module top;
  typedef logic [7:0] T;
  localparam int P = q::h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
