module child (input logic [cf(2):0] p);
  function automatic int cf(input int a); return 3; endfunction
  initial #1 $display("%m b=%0d p=%h", $bits(p), p);
endmodule
module top;
  logic [7:0] bus = 8'hA5;
  child u[1:0] (.p(bus));
  initial #2 $finish;
endmodule
