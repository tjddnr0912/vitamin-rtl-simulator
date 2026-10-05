package q;
  function automatic int f(input int a); return 3; endfunction
  function logic [31:0] h(input int x); return {f(2){1'b1}}; endfunction
endpackage
module top;
  import q::*;
  logic [31:0] v;
  initial begin v = h(0); $display("v=%h", v); #1 $finish; end
endmodule
