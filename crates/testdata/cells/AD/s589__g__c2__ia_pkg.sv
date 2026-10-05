package q;
  function automatic int cf(input int a); return 3; endfunction
endpackage
module child import q::*; (input logic [cf(2):0] p);
  initial #1 $display("%m b=%0d p=%h", $bits(p), p);
endmodule
module top;
  function automatic int cf(input int a);
    cf = 7;
  endfunction
  logic [7:0] bus = 8'hA5;
  child u[1:0] (.p(bus));
  initial #2 $finish;
endmodule
