package q;
  function automatic int f(input int a); return 3; endfunction
  function logic [31:0] h(input logic [15:0] x); return x[f(2):0]; endfunction
endpackage
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  localparam logic [31:0] P = q::h(16'hABCD);
  initial begin #1 $display("P=%h", P); $finish; end
endmodule
