module chk (input logic [f(2):0] s);
  function automatic int f(input int a); return 3; endfunction
  initial #1 $display("%m b=%0d s=%h", $bits(s), s);
endmodule
module tgt;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  logic [7:0] s = 8'h5a;
endmodule
module top;
  tgt t ();
  initial #2 $finish;
endmodule
bind tgt chk c (.s(s));
