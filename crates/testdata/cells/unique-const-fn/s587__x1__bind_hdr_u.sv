module chk #(parameter int W = f(2)) (input logic [7:0] s);
  function automatic int f(input int a); return 3; endfunction
  initial #1 $display("%m W=%0d", W);
endmodule
module tgt;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  logic [7:0] s = 8'h5a;
endmodule
module top;
  tgt t ();
  initial #2 $finish;
endmodule
bind tgt chk c (.s(s));
