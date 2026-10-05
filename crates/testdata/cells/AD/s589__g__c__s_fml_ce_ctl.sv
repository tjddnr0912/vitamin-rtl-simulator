package q;
  function automatic int f(input int a); return 3; endfunction
  function int h(input logic [f(2):0] x); return x; endfunction
endpackage
module top;
  localparam int P = q::h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
