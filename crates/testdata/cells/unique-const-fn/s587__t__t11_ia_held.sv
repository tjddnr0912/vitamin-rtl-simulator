module sub (input logic [f(2):0] p);
  function automatic int f(input int a);
    f = 3;
  endfunction
  initial #1 $display("%m b=%0d p=%h", $bits(p), p);
endmodule
module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [7:0] w = 8'h5a;
  sub u [1:0] (.p(w));
  initial #2 $finish;
endmodule
