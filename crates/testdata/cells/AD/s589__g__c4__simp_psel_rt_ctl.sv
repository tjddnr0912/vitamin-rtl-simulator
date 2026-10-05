package q;
  function automatic int f(input int a); return 3; endfunction
  function logic [31:0] h(input logic [15:0] x); return x[f(2):0]; endfunction
endpackage
module top;
  import q::h;
  logic [31:0] v;
  initial begin v = h(16'hABCD); $display("v=%h", v); #1 $finish; end
endmodule
