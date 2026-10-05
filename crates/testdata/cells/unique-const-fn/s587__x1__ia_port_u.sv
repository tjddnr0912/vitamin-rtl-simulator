module child (input logic [f(2):0] p);
  function automatic int f(input int a); return 3; endfunction
  initial #1 $display("%m b=%0d p=%h", $bits(p), p);
endmodule
module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [7:0] bus = 8'hA5;
  child u[1:0] (.p(bus));
  initial #2 $finish;
endmodule
