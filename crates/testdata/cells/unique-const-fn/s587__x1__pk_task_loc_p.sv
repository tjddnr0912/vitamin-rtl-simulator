package q;
  function automatic int f(input int a); return 3; endfunction
  task automatic tk(input int x, output int o);
    logic [f(2):0] t;
    t = x;
    o = t;
  endtask
endpackage
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  int v;
  initial begin q::tk(1000, v); $display("v=%0d", v); #1 $finish; end
endmodule
