module child (input logic [cf(2):0] p);
  function automatic int cf(input int a); return 3; endfunction
  initial #1 $display("%m b=%0d p=%h", $bits(p), p);
endmodule
module top;
  function automatic int cf(input int a);
    cf = 7;
  endfunction
  logic [7:0] bus = 8'hA5;
  child u (.p(bus[3:0]));
  initial #2 $finish;
endmodule
